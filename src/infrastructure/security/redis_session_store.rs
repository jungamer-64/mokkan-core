// src/infrastructure/security/redis_session_store.rs
use crate::application::ApplicationResult;
use crate::application::error::ApplicationError;
use crate::application::ports::session_revocation::SessionRevocationStore;
use async_trait::async_trait;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};
use redis::AsyncCommands;

#[derive(Clone)]
pub struct RedisSessionRevocationStore {
    pool: Pool,
}

impl RedisSessionRevocationStore {
    /// Create a new Redis backed session store from a redis URL (e.g. redis://:password@host:6379/0)
    pub fn from_url(url: &str) -> Result<Self, ApplicationError> {
        let cfg = DeadpoolConfig::from_url(url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl SessionRevocationStore for RedisSessionRevocationStore {
    async fn is_revoked(&self, session_id: &str) -> ApplicationResult<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("revoked:session:{}", session_id);
        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(exists)
    }

    async fn revoke(&self, session_id: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("revoked:session:{}", session_id);
        conn.set::<_, _, ()>(key, 1)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn get_min_token_version(&self, user_id: i64) -> ApplicationResult<Option<u32>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("min_token_version:{}", user_id);
        let val: Option<u32> = conn
            .get(key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(val)
    }

    async fn set_min_token_version(&self, user_id: i64, min_version: u32) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("min_token_version:{}", user_id);
        conn.set::<_, _, ()>(key, min_version)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn set_session_refresh_nonce(&self, session_id: &str, nonce: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("session_refresh_nonce:{}", session_id);
        conn.set::<_, _, ()>(key, nonce)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn get_session_refresh_nonce(&self, session_id: &str) -> ApplicationResult<Option<String>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("session_refresh_nonce:{}", session_id);
        let val: Option<String> = conn
            .get(key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(val)
    }

    async fn compare_and_swap_session_refresh_nonce(
        &self,
        session_id: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("session_refresh_nonce:{}", session_id);

        // Lua script: compare current value with expected, if equal set to new and return 1, else return 0
        let script = r#"
            local cur = redis.call('GET', KEYS[1])
            if cur == ARGV[1] then
                redis.call('SET', KEYS[1], ARGV[2])
                return 1
            else
                return 0
            end
        "#;

        let replaced: i32 = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(&key)
            .arg(expected)
            .arg(new_nonce)
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(replaced == 1)
    }
}

pub fn into_arc(store: RedisSessionRevocationStore) -> std::sync::Arc<dyn SessionRevocationStore> {
    std::sync::Arc::new(store)
}
