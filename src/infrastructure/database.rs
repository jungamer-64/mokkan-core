// src/infrastructure/database.rs
use sqlx::{PgPool, postgres::PgPoolOptions};

/// Initialize the `PostgreSQL` connection pool.
///
/// # Errors
///
/// Returns any `sqlx` error raised while connecting to the database.
pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .connect(database_url)
        .await
}

/// Run embedded SQL migrations against the configured pool.
///
/// # Errors
///
/// Returns any migration error reported by `sqlx`.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
