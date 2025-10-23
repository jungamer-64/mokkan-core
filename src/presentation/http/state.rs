// src/presentation/http/state.rs
use crate::application::services::ApplicationServices;
use std::sync::Arc;
use sqlx::PgPool;

#[derive(Clone)]
pub struct HttpState {
    pub services: Arc<ApplicationServices>,
    pub db_pool: PgPool,
}
