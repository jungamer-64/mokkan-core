// src/application/ports/authorization_code.rs
use crate::application::{AppResult, TokenSubject};
use crate::async_support::BoxFuture;
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

pub trait CodeStore: Send + Sync {
    fn create_code(&self, code: Code) -> BoxFuture<'_, AppResult<()>>;
    fn get_code<'a>(&'a self, code: &'a str) -> BoxFuture<'a, AppResult<Option<Code>>>;
    /// Consume (atomically remove) the code and return the stored value if present.
    fn consume_code<'a>(&'a self, code: &'a str) -> BoxFuture<'a, AppResult<Option<Code>>>;
}
