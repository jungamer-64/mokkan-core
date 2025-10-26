// src/infrastructure/security/redis_session_store.rs
use crate::application::ApplicationResult;
use crate::application::error::ApplicationError;
use crate::application::ports::session_revocation::SessionRevocationStore;
use async_trait::async_trait;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime, Connection};
use redis::AsyncCommands;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;

// TTL for used refresh-nonce markers (in seconds). 7 days by default.
const USED_NONCE_TTL_SECS: usize = 60 * 60 * 24 * 7; // 604800

// Lua script used to atomically rotate the refresh nonce and mark the old
// nonce as used (with a TTL). Extracted as a constant so helpers can reuse
// it without inflating function bodies (also helps with Lizard line-count).
const CAS_LUA_SCRIPT: &str = r#"
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

#[derive(Clone)]
pub struct RedisSessionRevocationStore {
    pool: Pool,
    /// Cached SHA for the compare-and-swap lua script. Loaded lazily.
    cas_script_sha: Arc<Mutex<Option<String>>>,
    /// Number of times the CAS script was loaded into Redis (SCRIPT LOAD).
    /// Used by tests to assert EVALSHA caching behavior.
    script_load_count: Arc<AtomicUsize>,
    /// TTL for used refresh nonce markers (seconds). Configurable via env REDIS_USED_NONCE_TTL_SECS.
    used_nonce_ttl_secs: usize,
}

impl RedisSessionRevocationStore {
    /// Create a new Redis backed session store from a redis URL (e.g. redis://:password@host:6379/0)
    pub fn from_url(url: &str) -> Result<Self, ApplicationError> {
        // Delegate to the options based constructor using environment defaults.
        let used_nonce_ttl_secs = std::env::var("REDIS_USED_NONCE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(USED_NONCE_TTL_SECS);

        let preload = std::env::var("REDIS_PRELOAD_CAS_SCRIPT").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false);

        Self::from_url_with_options(url, used_nonce_ttl_secs, preload)
    }

    /// Create a RedisSessionRevocationStore from a URL but allow configuration of
    /// the used-nonce TTL and whether to preload the CAS script at startup.
    pub fn from_url_with_options(
        url: &str,
        used_nonce_ttl_secs: usize,
        preload_cas_script: bool,
    ) -> Result<Self, ApplicationError> {
        let cfg = DeadpoolConfig::from_url(url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let store = Self {
            pool: pool.clone(),
            cas_script_sha: Arc::new(Mutex::new(None)),
            script_load_count: Arc::new(AtomicUsize::new(0)),
            used_nonce_ttl_secs,
        };

        if preload_cas_script {
            let pool_clone = pool.clone();
            let sha_clone = store.cas_script_sha.clone();
            tokio::spawn(async move {
                if let Ok(mut conn) = pool_clone.get().await {
                    match redis::cmd("SCRIPT").arg("LOAD").arg(CAS_LUA_SCRIPT).query_async::<String>(&mut conn).await {
                        Ok(sha) => {
                            let mut g = sha_clone.lock().await;
                            *g = Some(sha);
                            tracing::info!("preloaded redis CAS lua script");
                        }
                        Err(err) => tracing::warn!(error = %err, "failed to preload redis CAS lua script"),
                    }
                }
            });
        }

        Ok(store)
    }

    /// Helper that executes the CAS lua script using a cached SHA when possible.
    /// Loads the script (SCRIPT LOAD) on first use or when a NOSCRIPT is returned.
    pub async fn run_cas_script(
        &self,
        key: &str,
        used_key: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<i32> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        // 1) Try using the cached SHA (if present). This helper will clear the
        // cached value on NOSCRIPT and return None so we can fall back.
        if let Some(v) = self.try_cached_eval(&mut conn, key, used_key, expected, new_nonce).await? {
            return Ok(v);
        }

        // 2) Load the script and cache the SHA, then evaluate with the loaded SHA.
        let sha = self.load_script_and_cache(&mut conn).await?;
        let replaced = self.evalsha_by_sha(&mut conn, &sha, key, used_key, expected, new_nonce).await?;
        Ok(replaced)
    }

    async fn try_cached_eval(
        &self,
        conn: &mut Connection,
        key: &str,
        used_key: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<Option<i32>> {
        let mut sha_guard = self.cas_script_sha.lock().await;
        if let Some(sha) = sha_guard.clone() {
            let res: Result<i32, redis::RedisError> = redis::cmd("EVALSHA")
                .arg(sha)
                .arg(2)
                .arg(key)
                .arg(used_key)
                .arg(expected)
                .arg(new_nonce)
                .arg(self.used_nonce_ttl_secs)
                .query_async(conn)
                .await;

            match res {
                Ok(v) => return Ok(Some(v)),
                Err(err) => {
                    let msg = err.to_string();
                    if msg.contains("NOSCRIPT") {
                        *sha_guard = None;
                        return Ok(None);
                    } else {
                        return Err(ApplicationError::infrastructure(err.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn load_script_and_cache(&self, conn: &mut Connection) -> ApplicationResult<String> {
        let loaded_sha: String = redis::cmd("SCRIPT")
            .arg("LOAD")
            .arg(CAS_LUA_SCRIPT)
            .query_async(conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        // Count the script load for observability/testing.
        self.script_load_count.fetch_add(1, Ordering::SeqCst);

        let mut sha_guard = self.cas_script_sha.lock().await;
        *sha_guard = Some(loaded_sha.clone());
        Ok(loaded_sha)
    }

    /// Return the number of times we've called SCRIPT LOAD (test hook).
    pub fn script_loads(&self) -> usize {
        self.script_load_count.load(Ordering::SeqCst)
    }

    async fn evalsha_by_sha(
        &self,
        conn: &mut Connection,
        sha: &str,
        key: &str,
        used_key: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<i32> {
        let replaced: i32 = redis::cmd("EVALSHA")
            .arg(sha)
            .arg(2)
            .arg(key)
            .arg(used_key)
            .arg(expected)
            .arg(new_nonce)
            .arg(self.used_nonce_ttl_secs)
            .query_async(conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(replaced)
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
        let key = format!("session_refresh_nonce:{}", session_id);
        let used_key = format!("used_refresh_nonce:{}:{}", session_id, expected);

        let replaced = self
            .run_cas_script(&key, &used_key, expected, new_nonce)
            .await?;

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
            .arg(self.used_nonce_ttl_secs)
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

    async fn list_sessions_for_user_with_meta(&self, user_id: i64) -> ApplicationResult<Vec<crate::application::ports::session_revocation::SessionInfo>> {
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

        let mut out = Vec::with_capacity(sessions.len());
        for sid in sessions {
            let meta_key = format!("session:meta:{}", sid);
            // read fields individually to be robust to missing values
            let ua: Option<String> = conn
                .hget(&meta_key, "user_agent")
                .await
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
            let ip: Option<String> = conn
                .hget(&meta_key, "ip")
                .await
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
            let created_str: Option<String> = conn
                .hget(&meta_key, "created_at")
                .await
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
            let created_at_unix: i64 = created_str
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            let revoked_key = format!("revoked:session:{}", sid);
            let revoked: bool = conn
                .exists(&revoked_key)
                .await
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

            out.push(crate::application::ports::session_revocation::SessionInfo {
                user_id,
                session_id: sid,
                user_agent: ua,
                ip_address: ip,
                created_at_unix,
                revoked,
            });
        }

        Ok(out)
    }

    async fn set_session_metadata(
        &self,
        user_id: i64,
        session_id: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        created_at_unix: i64,
    ) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let user_sessions_key = format!("user_sessions:{}", user_id);
        conn.sadd::<_, _, ()>(user_sessions_key, session_id)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let meta_key = format!("session:meta:{}", session_id);
        // Use a single HSET invocation to reduce branching and RTTs. Store empty string
        // for optional fields when absent.
        let ua_val = user_agent.unwrap_or("");
        let ip_val = ip_address.unwrap_or("");

        let mut cmd = redis::cmd("HSET");
        cmd.arg(&meta_key)
            .arg("user_agent")
            .arg(ua_val)
            .arg("ip")
            .arg(ip_val)
            .arg("created_at")
            .arg(created_at_unix)
            .arg("user_id")
            .arg(user_id);

        let _: i32 = cmd
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(())
    }

    async fn get_session_metadata(&self, session_id: &str) -> ApplicationResult<Option<crate::application::ports::session_revocation::SessionInfo>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let meta_key = format!("session:meta:{}", session_id);
        // If the meta hash does not exist, return None
        let exists: bool = conn
            .exists(&meta_key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        if !exists {
            return Ok(None);
        }

        // Retrieve multiple fields at once
        let (ua, ip, created_opt, user_id_val): (Option<String>, Option<String>, Option<String>, Option<i64>) =
            conn
                .hget(&meta_key, ("user_agent", "ip", "created_at", "user_id"))
                .await
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let created_at_unix: i64 = created_opt
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let revoked_key = format!("revoked:session:{}", session_id);
        let revoked: bool = conn
            .exists(&revoked_key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(Some(crate::application::ports::session_revocation::SessionInfo {
            user_id: user_id_val.unwrap_or(0),
            session_id: session_id.to_string(),
            user_agent: ua,
            ip_address: ip,
            created_at_unix,
            revoked,
        }))
    }

    async fn delete_session_metadata(&self, session_id: &str) -> ApplicationResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let meta_key = format!("session:meta:{}", session_id);
        let _: () = conn
            .del(&meta_key)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        Ok(())
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
        // set using a Lua script. Instead of re-reading the set server-side
        // (SMEMBERS), pass the session IDs we already fetched as ARGV to the
        // script. This avoids a redundant read and keeps the operation atomic.
        // The script expects the key in KEYS[1] and session IDs in ARGV.
        let script = r#"
            if #ARGV == 0 then
                return 0
            end
            for i=1,#ARGV do
                local sid = ARGV[i]
                redis.call('SET', 'revoked:session:' .. sid, 1)
            end
            redis.call('DEL', KEYS[1])
            return #ARGV
        "#;

        let mut cmd = redis::cmd("EVAL");
        cmd.arg(script).arg(1).arg(&key);
        for sid in &sessions {
            cmd.arg(sid);
        }

        let _processed: i32 = cmd
            .query_async(&mut conn)
            .await
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        Ok(())
    }
}

pub fn into_arc(store: RedisSessionRevocationStore) -> std::sync::Arc<dyn SessionRevocationStore> {
    std::sync::Arc::new(store)
}
