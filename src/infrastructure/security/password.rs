// src/infrastructure/security/password.rs
use crate::application::{
    error::{AppError, AppResult},
    ports::security::PasswordHasher,
};
use crate::async_support::{BoxFuture, boxed};
use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};

#[derive(Default, Clone)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash<'a>(&'a self, password: &'a str) -> BoxFuture<'a, AppResult<String>> {
        let password = password.to_owned();
        boxed(async move {
            tokio::task::spawn_blocking(move || {
                let salt = SaltString::generate(&mut OsRng);
                let hash = Argon2::default()
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|err| AppError::infrastructure(err.to_string()))?;
                Ok(hash.to_string())
            })
            .await
            .map_err(|err| AppError::infrastructure(err.to_string()))?
        })
    }

    fn verify<'a>(
        &'a self,
        password: &'a str,
        expected_hash: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        let password = password.to_owned();
        let expected_hash = expected_hash.to_owned();
        boxed(async move {
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
        })
    }
}
