// src/presentation/http/state.rs
use crate::application::services::ApplicationServices;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct HttpState {
    pub services: Arc<ApplicationServices>,
    pub db_pool: PgPool,
}
