// src/infrastructure/security/session_store.rs
use crate::application::ApplicationResult;
use crate::application::ports::session_revocation::SessionRevocationStore;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::Arc;

#[derive(Default)]
pub struct InMemorySessionRevocationStore {
    revoked: Mutex<HashSet<String>>,
    min_versions: Mutex<HashMap<i64, u32>>,
    // per-session refresh nonce storage (session_id -> nonce)
    session_nonces: Mutex<HashMap<String, String>>,
}

impl InMemorySessionRevocationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionRevocationStore for InMemorySessionRevocationStore {
    async fn is_revoked(&self, session_id: &str) -> ApplicationResult<bool> {
        let guard = self.revoked.lock().unwrap();
        Ok(guard.contains(session_id))
    }

    async fn revoke(&self, session_id: &str) -> ApplicationResult<()> {
        let mut guard = self.revoked.lock().unwrap();
        guard.insert(session_id.to_string());
        Ok(())
    }

    async fn get_min_token_version(&self, user_id: i64) -> ApplicationResult<Option<u32>> {
        let guard = self.min_versions.lock().unwrap();
        Ok(guard.get(&user_id).cloned())
    }

    async fn set_min_token_version(&self, user_id: i64, min_version: u32) -> ApplicationResult<()> {
        let mut guard = self.min_versions.lock().unwrap();
        guard.insert(user_id, min_version);
        Ok(())
    }

    async fn set_session_refresh_nonce(&self, session_id: &str, nonce: &str) -> ApplicationResult<()> {
        let mut guard = self.session_nonces.lock().unwrap();
        guard.insert(session_id.to_string(), nonce.to_string());
        Ok(())
    }

    async fn get_session_refresh_nonce(&self, session_id: &str) -> ApplicationResult<Option<String>> {
        let guard = self.session_nonces.lock().unwrap();
        Ok(guard.get(session_id).cloned())
    }

    async fn compare_and_swap_session_refresh_nonce(
        &self,
        session_id: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<bool> {
        let mut guard = self.session_nonces.lock().unwrap();
        match guard.get(session_id) {
            Some(cur) if cur == expected => {
                guard.insert(session_id.to_string(), new_nonce.to_string());
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub fn into_arc(store: InMemorySessionRevocationStore) -> Arc<dyn SessionRevocationStore> {
    Arc::new(store)
}
