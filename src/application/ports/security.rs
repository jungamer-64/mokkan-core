// src/application/ports/security.rs
use crate::application::{
    ApplicationResult,
    dto::{AuthTokenDto, AuthenticatedUser, TokenSubject},
};
use async_trait::async_trait;

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> ApplicationResult<String>;
    async fn verify(&self, password: &str, expected_hash: &str) -> ApplicationResult<()>;
}

#[async_trait]
pub trait TokenManager: Send + Sync {
    async fn issue(&self, subject: TokenSubject) -> ApplicationResult<AuthTokenDto>;
    async fn authenticate(&self, token: &str) -> ApplicationResult<AuthenticatedUser>;
}
