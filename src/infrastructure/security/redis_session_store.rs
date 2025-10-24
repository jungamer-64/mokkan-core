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
}

pub fn into_arc(store: RedisSessionRevocationStore) -> std::sync::Arc<dyn SessionRevocationStore> {
    std::sync::Arc::new(store)
}
