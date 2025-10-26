// src/infrastructure/security/redis_session_store.rs
use crate::application::ApplicationResult;
use crate::application::error::ApplicationError;
use crate::application::ports::session_revocation::SessionRevocationStore;
use async_trait::async_trait;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};
use redis::AsyncCommands;

// TTL for used refresh-nonce markers (in seconds). 7 days by default.
const USED_NONCE_TTL_SECS: usize = 60 * 60 * 24 * 7; // 604800

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
        // used-nonce key for the presented (expected) nonce. We set this when a
        // successful rotation occurs so that later reuse can be detected.
        let used_key = format!("used_refresh_nonce:{}:{}", session_id, expected);

        // Lua script: Atomically compares the current session refresh nonce with the expected value.
        // If equal, replaces it with the new nonce, marks the presented nonce as used (with TTL), and returns 1.
        // Otherwise, returns 0. The TTL for the used nonce marker is provided as ARGV[3].
        let script = r#"
            local cur = redis.call('GET', KEYS[1])
            if cur == ARGV[1] then
                redis.call('SET', KEYS[1], ARGV[2])
                redis.call('SET', KEYS[2], 1)
                redis.call('EXPIRE', KEYS[2], ARGV[3])
                return 1
            else
                return 0
            end
        "#;

        let replaced: i32 = redis::cmd("EVAL")
            .arg(script)
            .arg(2)
            .arg(&key)
            .arg(&used_key)
            .arg(expected)
            .arg(new_nonce)
            .arg(USED_NONCE_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(replaced == 1)
    }

    async fn mark_session_refresh_nonce_used(&self, session_id: &str, nonce: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let used_key = format!("used_refresh_nonce:{}:{}", session_id, nonce);
        // default TTL
        conn.set::<_, _, ()>(&used_key, 1)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        // Use explicit EXPIRE command to avoid type inference issues with the
        // high-level helper and to use the shared TTL constant.
        let _: i32 = redis::cmd("EXPIRE")
            .arg(&used_key)
            .arg(USED_NONCE_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn is_session_refresh_nonce_used(&self, session_id: &str, nonce: &str) -> ApplicationResult<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let used_key = format!("used_refresh_nonce:{}:{}", session_id, nonce);
        let exists: bool = conn
            .exists(used_key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(exists)
    }

    async fn add_session_for_user(&self, user_id: i64, session_id: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("user_sessions:{}", user_id);
        conn.sadd::<_, _, ()>(key, session_id)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn remove_session_for_user(&self, user_id: i64, session_id: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("user_sessions:{}", user_id);
        conn.srem::<_, _, ()>(key, session_id)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
    }

    async fn list_sessions_for_user(&self, user_id: i64) -> ApplicationResult<Vec<String>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("user_sessions:{}", user_id);
        let members: Vec<String> = conn
            .smembers(key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(members)
    }

    async fn revoke_sessions_for_user(&self, user_id: i64) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let key = format!("user_sessions:{}", user_id);
        let sessions: Vec<String> = conn
            .smembers(&key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        if sessions.is_empty() {
            return Ok(());
        }

        // Atomically mark each session revoked and remove the user's session
        // set using a Lua script. This avoids races and ensures that clients
        // won't see partially applied state if the process is interrupted.
        // The script returns the number of sessions processed.
        let script = r#"
            local members = redis.call('SMEMBERS', KEYS[1])
            if next(members) == nil then
                return 0
            end
            for i=1,#members do
                local sid = members[i]
                redis.call('SET', 'revoked:session:' .. sid, 1)
            end
            redis.call('DEL', KEYS[1])
            return #members
        "#;

        let _processed: i32 = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(())
    }
}

pub fn into_arc(store: RedisSessionRevocationStore) -> std::sync::Arc<dyn SessionRevocationStore> {
    std::sync::Arc::new(store)
}
