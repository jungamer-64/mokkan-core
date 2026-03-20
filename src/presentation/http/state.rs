// src/presentation/http/state.rs
use crate::application::services::Registry;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct HttpContext {
    pub services: Arc<Registry>,
    pub db_pool: PgPool,
}
