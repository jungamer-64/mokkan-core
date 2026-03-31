// src/application/ports/security.rs
use crate::application::{AppResult, AuthTokenDto, AuthenticatedUser, TokenSubject};
use crate::async_support::BoxFuture;

pub trait PasswordHasher: Send + Sync {
    fn hash<'a>(&'a self, password: &'a str) -> BoxFuture<'a, AppResult<String>>;
    fn verify<'a>(
        &'a self,
        password: &'a str,
        expected_hash: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;
}

pub trait TokenManager: Send + Sync {
    fn issue(&self, subject: TokenSubject) -> BoxFuture<'_, AppResult<AuthTokenDto>>;
    fn authenticate<'a>(&'a self, token: &'a str) -> BoxFuture<'a, AppResult<AuthenticatedUser>>;
    /// Return a JSON Web Key Set (`JWKS`) or equivalent public-key representation.
    ///
    /// This is used to verify tokens issued by this `TokenManager` and powers
    /// the public keys endpoint.
    fn public_jwk(&self) -> BoxFuture<'_, AppResult<serde_json::Value>>;
}
