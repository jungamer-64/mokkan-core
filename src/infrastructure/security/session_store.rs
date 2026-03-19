// src/infrastructure/security/session_store.rs
use crate::application::ApplicationResult;
use crate::application::ports::session_revocation::{
    OpaqueRefreshTokenStore, RefreshNonceStore, RefreshTokenRecord, SessionMetadataStore,
    SessionRevocation, SessionRevocationStore, TokenVersionStore,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;

// Local helper struct to store session metadata in-memory
#[derive(Debug, Clone)]
struct SessionMeta {
    user_id: i64,
    user_agent: Option<String>,
    ip_address: Option<String>,
    created_at_unix: i64,
}

#[derive(Default)]
pub struct InMemorySessionRevocationStore {
    revoked: Mutex<HashSet<String>>,
    min_versions: Mutex<HashMap<i64, u32>>,
    // per-session refresh nonce storage (session_id -> nonce)
    session_nonces: Mutex<HashMap<String, String>>,
    // per-session used nonces (session_id -> set of used nonces)
    used_nonces: Mutex<HashMap<String, HashSet<String>>>,
    // per-user sessions (user_id -> set of session_ids)
    user_sessions: Mutex<HashMap<i64, HashSet<String>>>,
    // per-session metadata (session_id -> SessionMeta)
    session_meta: Mutex<HashMap<String, SessionMeta>>,
    // opaque refresh token record (token_id -> record)
    refresh_token_records: Mutex<HashMap<String, RefreshTokenRecord>>,
    // reverse index for refresh token cleanup (session_id -> token_ids)
    session_refresh_tokens: Mutex<HashMap<String, HashSet<String>>>,
}

impl InMemorySessionRevocationStore {
    pub fn new() -> Self {
        // Explicitly initialize each field to make construction obvious
        // and to avoid false-positive reviewer comments about missing
        // initialization. This is equivalent to `Self::default()` but
        // clearer to readers.
        Self {
            revoked: Mutex::new(HashSet::new()),
            min_versions: Mutex::new(HashMap::new()),
            session_nonces: Mutex::new(HashMap::new()),
            used_nonces: Mutex::new(HashMap::new()),
            user_sessions: Mutex::new(HashMap::new()),
            session_meta: Mutex::new(HashMap::new()),
            refresh_token_records: Mutex::new(HashMap::new()),
            session_refresh_tokens: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SessionRevocation for InMemorySessionRevocationStore {
    async fn is_revoked(&self, session_id: &str) -> ApplicationResult<bool> {
        let guard = self.revoked.lock().unwrap();
        Ok(guard.contains(session_id))
    }

    async fn revoke(&self, session_id: &str) -> ApplicationResult<()> {
        let mut guard = self.revoked.lock().unwrap();
        guard.insert(session_id.to_string());
        drop(guard);

        let mut tokens_guard = self.session_refresh_tokens.lock().unwrap();
        let token_ids = tokens_guard.remove(session_id);
        drop(tokens_guard);

        if let Some(token_ids) = token_ids {
            let mut records_guard = self.refresh_token_records.lock().unwrap();
            for token_id in token_ids {
                records_guard.remove(&token_id);
            }
        }
        Ok(())
    }

    async fn revoke_sessions_for_user(&self, user_id: i64) -> ApplicationResult<()> {
        let sessions = {
            let mut guard = self.user_sessions.lock().unwrap();
            match guard.remove(&user_id) {
                Some(s) => s.into_iter().collect::<Vec<_>>(),
                None => vec![],
            }
        };

        if !sessions.is_empty() {
            let mut revoked_guard = self.revoked.lock().unwrap();
            revoked_guard.extend(sessions.iter().cloned());
            drop(revoked_guard);

            let mut tokens_guard = self.session_refresh_tokens.lock().unwrap();
            let mut token_ids = Vec::new();
            for session_id in sessions {
                token_ids.extend(tokens_guard.remove(&session_id).into_iter().flatten());
            }
            drop(tokens_guard);

            let mut records_guard = self.refresh_token_records.lock().unwrap();
            for token_id in token_ids {
                records_guard.remove(&token_id);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl TokenVersionStore for InMemorySessionRevocationStore {
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

#[async_trait]
impl RefreshNonceStore for InMemorySessionRevocationStore {
    async fn set_session_refresh_nonce(
        &self,
        session_id: &str,
        nonce: &str,
    ) -> ApplicationResult<()> {
        let mut guard = self.session_nonces.lock().unwrap();
        guard.insert(session_id.to_string(), nonce.to_string());
        Ok(())
    }

    async fn get_session_refresh_nonce(
        &self,
        session_id: &str,
    ) -> ApplicationResult<Option<String>> {
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
                // mark the expected as used for later reuse detection
                let mut used_guard = self.used_nonces.lock().unwrap();
                let entry = used_guard.entry(session_id.to_string()).or_default();
                entry.insert(expected.to_string());
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn mark_session_refresh_nonce_used(
        &self,
        session_id: &str,
        nonce: &str,
    ) -> ApplicationResult<()> {
        let mut used_guard = self.used_nonces.lock().unwrap();
        let entry = used_guard.entry(session_id.to_string()).or_default();
        entry.insert(nonce.to_string());
        Ok(())
    }

    async fn is_session_refresh_nonce_used(
        &self,
        session_id: &str,
        nonce: &str,
    ) -> ApplicationResult<bool> {
        let used_guard = self.used_nonces.lock().unwrap();
        Ok(used_guard
            .get(session_id)
            .map(|s| s.contains(nonce))
            .unwrap_or(false))
    }
}

#[async_trait]
impl SessionMetadataStore for InMemorySessionRevocationStore {
    async fn add_session_for_user(&self, user_id: i64, session_id: &str) -> ApplicationResult<()> {
        let mut guard = self.user_sessions.lock().unwrap();
        let entry = guard.entry(user_id).or_default();
        entry.insert(session_id.to_string());
        Ok(())
    }

    async fn set_session_metadata(
        &self,
        user_id: i64,
        session_id: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        created_at_unix: i64,
    ) -> ApplicationResult<()> {
        // ensure session is tracked for the user
        {
            let mut guard = self.user_sessions.lock().unwrap();
            let entry = guard.entry(user_id).or_default();
            entry.insert(session_id.to_string());
        }

        let mut meta_guard = self.session_meta.lock().unwrap();
        meta_guard.insert(
            session_id.to_string(),
            SessionMeta {
                user_id,
                user_agent: user_agent.map(|s| s.to_string()),
                ip_address: ip_address.map(|s| s.to_string()),
                created_at_unix,
            },
        );

        Ok(())
    }

    async fn remove_session_for_user(
        &self,
        user_id: i64,
        session_id: &str,
    ) -> ApplicationResult<()> {
        let mut guard = self.user_sessions.lock().unwrap();
        if let Some(set) = guard.get_mut(&user_id) {
            set.remove(session_id);
        }
        Ok(())
    }

    async fn delete_session_metadata(&self, session_id: &str) -> ApplicationResult<()> {
        let mut meta_guard = self.session_meta.lock().unwrap();
        meta_guard.remove(session_id);
        Ok(())
    }

    async fn list_sessions_for_user(&self, user_id: i64) -> ApplicationResult<Vec<String>> {
        let guard = self.user_sessions.lock().unwrap();
        if let Some(set) = guard.get(&user_id) {
            Ok(set.iter().cloned().collect())
        } else {
            Ok(vec![])
        }
    }

    async fn list_sessions_for_user_with_meta(
        &self,
        user_id: i64,
    ) -> ApplicationResult<Vec<crate::application::ports::session_revocation::SessionInfo>> {
        let sessions = {
            let guard = self.user_sessions.lock().unwrap();
            match guard.get(&user_id) {
                Some(set) => set.iter().cloned().collect::<Vec<_>>(),
                None => vec![],
            }
        };

        let mut out = Vec::with_capacity(sessions.len());
        let meta_guard = self.session_meta.lock().unwrap();
        let revoked_guard = self.revoked.lock().unwrap();

        for sid in sessions {
            if let Some(m) = meta_guard.get(&sid) {
                out.push(crate::application::ports::session_revocation::SessionInfo {
                    user_id: m.user_id,
                    session_id: sid.clone(),
                    user_agent: m.user_agent.clone(),
                    ip_address: m.ip_address.clone(),
                    created_at_unix: m.created_at_unix,
                    revoked: revoked_guard.contains(&sid),
                });
            } else {
                out.push(crate::application::ports::session_revocation::SessionInfo {
                    user_id,
                    session_id: sid.clone(),
                    user_agent: None,
                    ip_address: None,
                    created_at_unix: 0,
                    revoked: revoked_guard.contains(&sid),
                });
            }
        }

        Ok(out)
    }

    async fn get_session_metadata(
        &self,
        session_id: &str,
    ) -> ApplicationResult<Option<crate::application::ports::session_revocation::SessionInfo>> {
        let meta_guard = self.session_meta.lock().unwrap();
        if let Some(m) = meta_guard.get(session_id) {
            let revoked_guard = self.revoked.lock().unwrap();
            Ok(Some(
                crate::application::ports::session_revocation::SessionInfo {
                    user_id: m.user_id,
                    session_id: session_id.to_string(),
                    user_agent: m.user_agent.clone(),
                    ip_address: m.ip_address.clone(),
                    created_at_unix: m.created_at_unix,
                    revoked: revoked_guard.contains(session_id),
                },
            ))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl OpaqueRefreshTokenStore for InMemorySessionRevocationStore {
    async fn store_refresh_token_record(
        &self,
        token_id: &str,
        record: &RefreshTokenRecord,
    ) -> ApplicationResult<()> {
        let mut records_guard = self.refresh_token_records.lock().unwrap();
        records_guard.insert(token_id.to_string(), record.clone());
        drop(records_guard);

        let mut session_tokens_guard = self.session_refresh_tokens.lock().unwrap();
        session_tokens_guard
            .entry(record.session_id.clone())
            .or_default()
            .insert(token_id.to_string());
        Ok(())
    }

    async fn get_refresh_token_record(
        &self,
        token_id: &str,
    ) -> ApplicationResult<Option<RefreshTokenRecord>> {
        let guard = self.refresh_token_records.lock().unwrap();
        Ok(guard.get(token_id).cloned())
    }

    async fn delete_refresh_token_record(&self, token_id: &str) -> ApplicationResult<()> {
        let mut records_guard = self.refresh_token_records.lock().unwrap();
        if let Some(record) = records_guard.remove(token_id) {
            drop(records_guard);
            let mut session_tokens_guard = self.session_refresh_tokens.lock().unwrap();
            if let Some(token_ids) = session_tokens_guard.get_mut(&record.session_id) {
                token_ids.remove(token_id);
                if token_ids.is_empty() {
                    session_tokens_guard.remove(&record.session_id);
                }
            }
        }

        Ok(())
    }

    async fn delete_refresh_tokens_for_session(&self, session_id: &str) -> ApplicationResult<()> {
        let mut session_tokens_guard = self.session_refresh_tokens.lock().unwrap();
        if let Some(token_ids) = session_tokens_guard.remove(session_id) {
            drop(session_tokens_guard);
            let mut records_guard = self.refresh_token_records.lock().unwrap();
            for token_id in token_ids {
                records_guard.remove(&token_id);
            }
        }

        Ok(())
    }
}

pub fn into_arc(store: InMemorySessionRevocationStore) -> Arc<dyn SessionRevocationStore> {
    Arc::new(store)
}
