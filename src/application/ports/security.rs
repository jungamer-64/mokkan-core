// src/application/ports/security.rs
use crate::application::{AppResult, AuthTokenDto, AuthenticatedUser, TokenSubject};
use async_trait::async_trait;

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> AppResult<String>;
    async fn verify(&self, password: &str, expected_hash: &str) -> AppResult<()>;
}

#[async_trait]
pub trait TokenManager: Send + Sync {
    async fn issue(&self, subject: TokenSubject) -> AppResult<AuthTokenDto>;
    async fn authenticate(&self, token: &str) -> AppResult<AuthenticatedUser>;
    /// Return a JSON Web Key Set (`JWKS`) or equivalent public-key representation.
    ///
    /// This is used to verify tokens issued by this `TokenManager` and powers
    /// the public keys endpoint.
    async fn public_jwk(&self) -> AppResult<serde_json::Value>;
}
