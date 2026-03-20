// src/application/ports/authorization_code.rs
use crate::application::{AppResult, TokenSubject};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Code {
    pub code: String,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub subject: TokenSubject,
    pub scope: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait CodeStore: Send + Sync {
    async fn create_code(&self, code: Code) -> AppResult<()>;
    async fn get_code(&self, code: &str) -> AppResult<Option<Code>>;
    /// Consume (atomically remove) the code and return the stored value if present.
    async fn consume_code(&self, code: &str) -> AppResult<Option<Code>>;
}
