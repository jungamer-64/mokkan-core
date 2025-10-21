mod application;
mod config;
mod domain;
mod infrastructure;
mod presentation;

use crate::application::{
    ports::{
        security::{PasswordHasher, TokenManager},
        time::Clock,
        util::SlugGenerator,
    },
    services::ApplicationServices,
};
use crate::config::AppConfig;
use crate::domain::{
    article::{ArticleReadRepository, ArticleWriteRepository},
    user::UserRepository,
};
use crate::infrastructure::{
    database,
    repositories::{
        SqliteArticleReadRepository, SqliteArticleWriteRepository, SqliteUserRepository,
    },
    security::{password::Argon2PasswordHasher, token::BiscuitTokenManager},
    time::SystemClock,
    util::DefaultSlugGenerator,
};
use crate::presentation::http::{routes::build_router, state::HttpState};
use anyhow::Result;
use axum::{ServiceExt, body::Body};
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    if let Err(err) = bootstrap().await {
        tracing::error!(error = %err, "fatal error");
        eprintln!("fatal error: {err}");
        std::process::exit(1);
    }
}

async fn bootstrap() -> Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;

    let pool = database::init_pool(config.database_url()).await?;
    database::run_migrations(&pool).await?;
    let pool = Arc::new(pool);

    let user_repo: Arc<dyn UserRepository> = Arc::new(SqliteUserRepository::new(Arc::clone(&pool)));
    let article_write_repo: Arc<dyn ArticleWriteRepository> =
        Arc::new(SqliteArticleWriteRepository::new(Arc::clone(&pool)));
    let article_read_repo: Arc<dyn ArticleReadRepository> =
        Arc::new(SqliteArticleReadRepository::new(Arc::clone(&pool)));

    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher::default());
    let token_manager_impl =
        BiscuitTokenManager::new(config.biscuit_private_key(), config.token_ttl())?;
    let token_manager: Arc<dyn TokenManager> = Arc::new(token_manager_impl);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::default());
    let slugger: Arc<dyn SlugGenerator> = Arc::new(DefaultSlugGenerator::default());

    let services = Arc::new(ApplicationServices::new(
        Arc::clone(&user_repo),
        Arc::clone(&article_write_repo),
        Arc::clone(&article_read_repo),
        Arc::clone(&password_hasher),
        Arc::clone(&token_manager),
        Arc::clone(&clock),
        Arc::clone(&slugger),
    ));

    let state = HttpState {
        services: Arc::clone(&services),
    };

    let app = build_router(state);
    let service = app.into_service::<Body>().into_make_service();

    let listener = tokio::net::TcpListener::bind(config.listen_addr()).await?;
    let address: SocketAddr = listener.local_addr()?;
    tracing::info!("listening on {address}");

    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
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
