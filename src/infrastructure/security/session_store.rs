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
}

pub fn into_arc(store: InMemorySessionRevocationStore) -> Arc<dyn SessionRevocationStore> {
    Arc::new(store)
}
