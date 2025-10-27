// src/application/ports/authorization_code.rs
use crate::application::{ApplicationResult, dto::TokenSubject};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AuthorizationCode {
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
pub trait AuthorizationCodeStore: Send + Sync {
    async fn create_code(&self, code: AuthorizationCode) -> ApplicationResult<()>;
    async fn get_code(&self, code: &str) -> ApplicationResult<Option<AuthorizationCode>>;
    /// Consume (atomically remove) the code and return the stored value if present.
    async fn consume_code(&self, code: &str) -> ApplicationResult<Option<AuthorizationCode>>;
}
