// src/main.rs
use anyhow::Result;
use axum::{ServiceExt, body::Body};
use mokkan_core::application::ports::util::SlugGenerator;
use mokkan_core::application::{
    ports::{
        security::{PasswordHasher, TokenManager},
        time::Clock,
    },
    services::{ApplicationServices, ApplicationDependencies},
};
use mokkan_core::infrastructure::security::authorization_code_store::into_arc as into_auth_code_store;
use mokkan_core::infrastructure::security::authorization_code_store::InMemoryAuthorizationCodeStore;
use mokkan_core::config::AppConfig;
use mokkan_core::domain::{
    article::{ArticleReadRepository, ArticleRevisionRepository, ArticleWriteRepository},
    user::UserRepository,
};
use mokkan_core::infrastructure::{
    database,
    repositories::{
        PostgresArticleReadRepository, PostgresArticleRevisionRepository,
        PostgresArticleWriteRepository, PostgresUserRepository,
            PostgresAuditLogRepository,
    },
    security::{password::Argon2PasswordHasher, token::BiscuitTokenManager},
    time::SystemClock,
    util::DefaultSlugGenerator,
};
use mokkan_core::infrastructure::security::session_store::InMemorySessionRevocationStore;
use mokkan_core::infrastructure::security::redis_session_store::RedisSessionRevocationStore;
use mokkan_core::application::ports::session_revocation::SessionRevocationStore;
use mokkan_core::presentation::http::{routes::build_router, state::HttpState};
use std::{env, net::SocketAddr, sync::Arc};
use sqlx::PgPool;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Allow triggering the OpenAPI snapshot generation via environment variable in CI
    // or development workflows. Avoid relying on command-line args for this control.
    if std::env::var("OPENAPI_SNAPSHOT").as_deref() == Ok("1") {
        dotenvy::dotenv().ok();
        if let Err(err) = mokkan_core::presentation::http::openapi::write_openapi_snapshot() {
            eprintln!("failed to write OpenAPI snapshot: {err}");
            std::process::exit(1);
        }
        let output_path = env::var("OPENAPI_SNAPSHOT_PATH")
            .unwrap_or_else(|_| "backend/spec/openapi.json".to_string());
        println!("OpenAPI snapshot written to {output_path}");
        return;
    }

    if let Err(err) = bootstrap().await {
        tracing::error!(error = %err, "fatal error");
        eprintln!("fatal error: {err}");
        std::process::exit(1);
    }
}

async fn bootstrap() -> Result<()> {
    init_tracing();

    let (config, pool) = init_config_and_db().await?;

    let (_services, state) = build_services_and_state(&pool, &config)?;

    let app = build_router(state);
    if let Err(err) = mokkan_core::presentation::http::openapi::write_openapi_snapshot() {
        tracing::warn!(error = %err, "failed to write OpenAPI snapshot");
    }
    let service = app.into_service::<Body>().into_make_service();

    let listener = tokio::net::TcpListener::bind(config.listen_addr()).await?;
    let address: SocketAddr = listener.local_addr()?;
    tracing::info!("listening on {address}");

    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn init_config_and_db() -> Result<(AppConfig, PgPool)> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;

    let pool = database::init_pool(config.database_url()).await?;
    database::run_migrations(&pool).await?;

    Ok((config, pool))
}

fn init_session_store(config: &AppConfig) -> Arc<dyn SessionRevocationStore> {
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        match RedisSessionRevocationStore::from_url_with_options(&redis_url, config.redis_used_nonce_ttl_secs(), config.redis_preload_cas_script()) {
            Ok(store) => Arc::new(store),
            Err(err) => {
                tracing::error!(error = %err, "failed to initialise redis session store, falling back to in-memory store");
                Arc::new(InMemorySessionRevocationStore::new())
            }
        }
    } else {
        Arc::new(InMemorySessionRevocationStore::new())
    }
}

fn build_services_and_state(
    pool: &PgPool,
    config: &AppConfig,
) -> Result<(Arc<ApplicationServices>, HttpState)> {
    let user_repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
    let article_write_repo: Arc<dyn ArticleWriteRepository> =
        Arc::new(PostgresArticleWriteRepository::new(pool.clone()));
    let article_read_repo: Arc<dyn ArticleReadRepository> =
        Arc::new(PostgresArticleReadRepository::new(pool.clone()));
    let article_revision_repo: Arc<dyn ArticleRevisionRepository> =
        Arc::new(PostgresArticleRevisionRepository::new(pool.clone()));

    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher::default());
    let token_manager_impl =
        BiscuitTokenManager::new(config.biscuit_private_key(), config.token_ttl())?;
    let token_manager: Arc<dyn TokenManager> = Arc::new(token_manager_impl);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::default());
    let slugger: Arc<dyn SlugGenerator> = Arc::new(DefaultSlugGenerator::default());

    let audit_log_repo: Arc<dyn mokkan_core::domain::audit::repository::AuditLogRepository> =
        Arc::new(PostgresAuditLogRepository::new(pool.clone()));

    let session_store = init_session_store(config);
    let auth_code_store = into_auth_code_store(InMemoryAuthorizationCodeStore::new());

    let deps = ApplicationDependencies {
        user_repo: Arc::clone(&user_repo),
        article_write_repo: Arc::clone(&article_write_repo),
        article_read_repo: Arc::clone(&article_read_repo),
        article_revision_repo: Arc::clone(&article_revision_repo),
        audit_log_repo: Arc::clone(&audit_log_repo),
    };

    let services = Arc::new(ApplicationServices::new(
        deps,
        Arc::clone(&password_hasher),
        Arc::clone(&token_manager),
        Arc::clone(&session_store),
        Arc::clone(&auth_code_store),
        Arc::clone(&clock),
        Arc::clone(&slugger),
    ));

    let state = HttpState {
        services: Arc::clone(&services),
        db_pool: pool.clone(),
    };

    Ok((services, state))
}

fn init_tracing() {
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .unwrap_or_else(|| "info,tower_http=info,sqlx=warn".to_string());

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(env_filter))
        .with(tracing_subscriber::fmt::layer());

    if subscriber.try_init().is_err() {
        tracing::warn!("tracing subscriber already initialised");
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    tracing::info!("shutdown signal received");
}
