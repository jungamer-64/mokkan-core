// src/infrastructure/security/password.rs
use crate::application::{
    error::{AppError, AppResult},
    ports::security::PasswordHasher,
};
use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use async_trait::async_trait;

#[derive(Default, Clone)]
pub struct Argon2PasswordHasher;

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: &str) -> AppResult<String> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|err| AppError::infrastructure(err.to_string()))?;
            Ok(hash.to_string())
        })
        .await
        .map_err(|err| AppError::infrastructure(err.to_string()))?
    }

    async fn verify(&self, password: &str, expected_hash: &str) -> AppResult<()> {
        let password = password.to_owned();
        let expected_hash = expected_hash.to_owned();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let parsed = PasswordHash::new(&expected_hash)
                .map_err(|err| AppError::infrastructure(err.to_string()))?;
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .map_err(|_| AppError::unauthorized("invalid credentials"))
        })
        .await
        .map_err(|err| AppError::infrastructure(err.to_string()))??;
        Ok(())
    }
}
