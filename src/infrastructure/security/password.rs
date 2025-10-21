use crate::application::{
    error::{ApplicationError, ApplicationResult},
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
    async fn hash(&self, password: &str) -> ApplicationResult<String> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))
        })
        .await
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?
    }

    async fn verify(&self, password: &str, expected_hash: &str) -> ApplicationResult<()> {
        let password = password.to_owned();
        let expected_hash = expected_hash.to_owned();
        tokio::task::spawn_blocking(move || -> Result<(), ApplicationError> {
            let parsed = PasswordHash::new(&expected_hash)
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .map_err(|_| ApplicationError::unauthorized("invalid credentials"))
        })
        .await
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))??;
        Ok(())
    }
}
