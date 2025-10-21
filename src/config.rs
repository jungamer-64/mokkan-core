use std::{env, time::Duration};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppConfig {
    database_url: String,
    listen_addr: String,
    biscuit_private_key: String,
    token_ttl: Duration,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cms".to_string());
        let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let biscuit_private_key = env::var("BISCUIT_ROOT_PRIVATE_KEY")
            .map_err(|_| ConfigError::Missing("BISCUIT_ROOT_PRIVATE_KEY"))?;

        if biscuit_private_key.len() != 64 {
            return Err(ConfigError::Invalid(
                "BISCUIT_ROOT_PRIVATE_KEY must be a 32-byte hex string".into(),
            ));
        }

        let token_ttl = env::var("TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(3_600);

        Ok(Self {
            database_url,
            listen_addr,
            biscuit_private_key,
            token_ttl: Duration::from_secs(token_ttl),
        })
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn listen_addr(&self) -> &str {
        &self.listen_addr
    }

    pub fn biscuit_private_key(&self) -> &str {
        &self.biscuit_private_key
    }

    pub fn token_ttl(&self) -> Duration {
        self.token_ttl
    }
}
