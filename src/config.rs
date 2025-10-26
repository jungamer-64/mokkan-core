// src/config.rs
use std::{env, time::Duration};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppConfig {
    database_url: String,
    listen_addr: String,
    biscuit_private_key: String,
    token_ttl: Duration,
    allowed_origins: Vec<String>,
    // Redis-related runtime options
    redis_used_nonce_ttl_secs: usize,
    redis_preload_cas_script: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5432/cms".into()
}

fn default_listen_addr() -> String {
    "127.0.0.1:8080".into()
}

fn default_token_ttl() -> u64 {
    3600
}

fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:3000".into()]
}

impl AppConfig {
    /// Build configuration from environment variables. Uses sensible defaults
    /// for optional values and validates required keys.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Allow dotenv files to populate env vars when present.
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| default_database_url());
        let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| default_listen_addr());
        let biscuit_private_key = env::var("BISCUIT_ROOT_PRIVATE_KEY")
            .map_err(|_| ConfigError::Missing("BISCUIT_ROOT_PRIVATE_KEY"))?;

        if biscuit_private_key.len() != 64 {
            return Err(ConfigError::Invalid(
                "BISCUIT_ROOT_PRIVATE_KEY must be a 32-byte hex string".into(),
            ));
        }

        let token_ttl_secs = env::var("TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or_else(default_token_ttl);

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_else(default_allowed_origins);

        let redis_used_nonce_ttl_secs = env::var("REDIS_USED_NONCE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(60 * 60 * 24 * 7);

        let redis_preload_cas_script = env::var("REDIS_PRELOAD_CAS_SCRIPT")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            database_url,
            listen_addr,
            biscuit_private_key,
            token_ttl: Duration::from_secs(token_ttl_secs),
            allowed_origins,
            redis_used_nonce_ttl_secs,
            redis_preload_cas_script,
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

    /// Return the allowed CORS origins as configured (cached on AppConfig).
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Backwards-compatible helper used by router construction in a few places
    /// where creating a full `AppConfig` is unnecessary for tests.
    pub fn allowed_origins_from_env() -> Vec<String> {
        env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_else(default_allowed_origins)
    }

    /// TTL for used refresh nonces (seconds)
    pub fn redis_used_nonce_ttl_secs(&self) -> usize {
        self.redis_used_nonce_ttl_secs
    }

    /// Whether to attempt preloading CAS lua scripts at startup
    pub fn redis_preload_cas_script(&self) -> bool {
        self.redis_preload_cas_script
    }

    /// Determine the issuer URL for OIDC discovery. Prefer explicit env var
    /// `OIDC_ISSUER` if present; otherwise derive a sensible default using
    /// the configured listen address.
    pub fn oidc_issuer_from_env() -> String {
        std::env::var("OIDC_ISSUER").unwrap_or_else(|_| format!("http://{}", default_listen_addr()))
    }
}
