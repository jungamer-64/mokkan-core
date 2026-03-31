// src/infrastructure/security/session_store.rs
use crate::application::AppResult;
use crate::application::ports::session_revocation::{
    OpaqueRefreshTokenStore, RefreshNonceStore, RefreshTokenRecord, Revocation,
    SessionMetadataStore, Store, TokenVersionStore,
};
use crate::async_support::{BoxFuture, boxed};
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
#[must_use]
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

    fn delete_refresh_tokens_for_session_inner(&self, session_id: &str) {
        let token_ids = {
            let mut tokens_guard = self.session_refresh_tokens.lock().unwrap();
            tokens_guard.remove(session_id)
        };

        if let Some(token_ids) = token_ids {
            let mut records_guard = self.refresh_token_records.lock().unwrap();
            for token_id in token_ids {
                records_guard.remove(&token_id);
            }
        }
    }

    fn delete_refresh_tokens_for_sessions<I>(&self, session_ids: I)
    where
        I: IntoIterator<Item = String>,
    {
        let mut tokens_guard = self.session_refresh_tokens.lock().unwrap();
        let token_ids = session_ids
            .into_iter()
            .flat_map(|session_id| tokens_guard.remove(&session_id).into_iter().flatten())
            .collect::<Vec<_>>();
        drop(tokens_guard);

        if !token_ids.is_empty() {
            let mut records_guard = self.refresh_token_records.lock().unwrap();
            for token_id in token_ids {
                records_guard.remove(&token_id);
            }
        }
    }

    fn session_info_from_meta(
        session_id: String,
        fallback_user_id: i64,
        meta: Option<&SessionMeta>,
        revoked: bool,
    ) -> crate::application::ports::session_revocation::SessionInfo {
        crate::application::ports::session_revocation::SessionInfo {
            user_id: meta.map_or(fallback_user_id, |value| value.user_id),
            session_id,
            user_agent: meta.and_then(|value| value.user_agent.clone()),
            ip_address: meta.and_then(|value| value.ip_address.clone()),
            created_at_unix: meta.map_or(0, |value| value.created_at_unix),
            revoked,
        }
    }
}

impl Revocation for InMemorySessionRevocationStore {
    fn is_revoked<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<bool>> {
        boxed(async move {
            let guard = self.revoked.lock().unwrap();
            Ok(guard.contains(session_id))
        })
    }

    fn revoke<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut guard = self.revoked.lock().unwrap();
            guard.insert(session_id.to_string());
            drop(guard);
            self.delete_refresh_tokens_for_session_inner(session_id);
            Ok(())
        })
    }

    fn revoke_sessions_for_user(&self, user_id: i64) -> BoxFuture<'_, AppResult<()>> {
        boxed(async move {
            let sessions = {
                let mut guard = self.user_sessions.lock().unwrap();
                guard
                    .remove(&user_id)
                    .map_or_else(Vec::new, |s| s.into_iter().collect::<Vec<_>>())
            };

            if !sessions.is_empty() {
                let mut revoked_guard = self.revoked.lock().unwrap();
                revoked_guard.extend(sessions.iter().cloned());
                drop(revoked_guard);
                self.delete_refresh_tokens_for_sessions(sessions);
            }

            Ok(())
        })
    }
}

impl TokenVersionStore for InMemorySessionRevocationStore {
    fn get_min_token_version(&self, user_id: i64) -> BoxFuture<'_, AppResult<Option<u32>>> {
        boxed(async move {
            let guard = self.min_versions.lock().unwrap();
            Ok(guard.get(&user_id).copied())
        })
    }

    fn set_min_token_version(
        &self,
        user_id: i64,
        min_version: u32,
    ) -> BoxFuture<'_, AppResult<()>> {
        boxed(async move {
            let mut guard = self.min_versions.lock().unwrap();
            guard.insert(user_id, min_version);
            drop(guard);
            Ok(())
        })
    }
}

impl RefreshNonceStore for InMemorySessionRevocationStore {
    fn set_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut guard = self.session_nonces.lock().unwrap();
            guard.insert(session_id.to_string(), nonce.to_string());
            drop(guard);
            Ok(())
        })
    }

    fn get_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<String>>> {
        boxed(async move {
            let guard = self.session_nonces.lock().unwrap();
            Ok(guard.get(session_id).cloned())
        })
    }

    fn compare_and_swap_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
        expected: &'a str,
        new_nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<bool>> {
        boxed(async move {
            let swapped = {
                let mut guard = self.session_nonces.lock().unwrap();
                match guard.get(session_id) {
                    Some(cur) if cur == expected => {
                        guard.insert(session_id.to_string(), new_nonce.to_string());
                        true
                    }
                    _ => false,
                }
            };

            if swapped {
                let mut used_guard = self.used_nonces.lock().unwrap();
                used_guard
                    .entry(session_id.to_string())
                    .or_default()
                    .insert(expected.to_string());
                drop(used_guard);
            }

            Ok(swapped)
        })
    }

    fn mark_session_refresh_nonce_used<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut used_guard = self.used_nonces.lock().unwrap();
            used_guard
                .entry(session_id.to_string())
                .or_default()
                .insert(nonce.to_string());
            drop(used_guard);
            Ok(())
        })
    }

    fn is_session_refresh_nonce_used<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<bool>> {
        boxed(async move {
            let used_guard = self.used_nonces.lock().unwrap();
            Ok(used_guard
                .get(session_id)
                .is_some_and(|set| set.contains(nonce)))
        })
    }
}

impl SessionMetadataStore for InMemorySessionRevocationStore {
    fn add_session_for_user<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut guard = self.user_sessions.lock().unwrap();
            guard
                .entry(user_id)
                .or_default()
                .insert(session_id.to_string());
            drop(guard);
            Ok(())
        })
    }

    fn set_session_metadata<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
        user_agent: Option<&'a str>,
        ip_address: Option<&'a str>,
        created_at_unix: i64,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            // ensure session is tracked for the user
            {
                let mut guard = self.user_sessions.lock().unwrap();
                guard
                    .entry(user_id)
                    .or_default()
                    .insert(session_id.to_string());
            }

            let mut meta_guard = self.session_meta.lock().unwrap();
            meta_guard.insert(
                session_id.to_string(),
                SessionMeta {
                    user_id,
                    user_agent: user_agent.map(std::string::ToString::to_string),
                    ip_address: ip_address.map(std::string::ToString::to_string),
                    created_at_unix,
                },
            );
            drop(meta_guard);

            Ok(())
        })
    }

    fn remove_session_for_user<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut guard = self.user_sessions.lock().unwrap();
            if let Some(set) = guard.get_mut(&user_id) {
                set.remove(session_id);
            }
            drop(guard);
            Ok(())
        })
    }

    fn delete_session_metadata<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut meta_guard = self.session_meta.lock().unwrap();
            meta_guard.remove(session_id);
            drop(meta_guard);
            Ok(())
        })
    }

    fn list_sessions_for_user(&self, user_id: i64) -> BoxFuture<'_, AppResult<Vec<String>>> {
        boxed(async move {
            let guard = self.user_sessions.lock().unwrap();
            let sessions = guard
                .get(&user_id)
                .map_or_else(Vec::new, |set| set.iter().cloned().collect());
            drop(guard);
            Ok(sessions)
        })
    }

    fn list_sessions_for_user_with_meta(
        &self,
        user_id: i64,
    ) -> BoxFuture<'_, AppResult<Vec<crate::application::ports::session_revocation::SessionInfo>>>
    {
        boxed(async move {
            let sessions = {
                let guard = self.user_sessions.lock().unwrap();
                guard
                    .get(&user_id)
                    .map_or_else(Vec::new, |set| set.iter().cloned().collect::<Vec<_>>())
            };

            let mut out = Vec::with_capacity(sessions.len());
            let meta_guard = self.session_meta.lock().unwrap();
            let revoked_guard = self.revoked.lock().unwrap();

            for sid in sessions {
                out.push(Self::session_info_from_meta(
                    sid.clone(),
                    user_id,
                    meta_guard.get(&sid),
                    revoked_guard.contains(&sid),
                ));
            }

            drop(revoked_guard);
            drop(meta_guard);
            Ok(out)
        })
    }

    fn get_session_metadata<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<crate::application::ports::session_revocation::SessionInfo>>>
    {
        boxed(async move {
            let meta_guard = self.session_meta.lock().unwrap();
            let meta = meta_guard.get(session_id).cloned();
            drop(meta_guard);

            let Some(meta) = meta else {
                return Ok(None);
            };

            let revoked_guard = self.revoked.lock().unwrap();
            let session = Self::session_info_from_meta(
                session_id.to_string(),
                meta.user_id,
                Some(&meta),
                revoked_guard.contains(session_id),
            );
            drop(revoked_guard);
            Ok(Some(session))
        })
    }
}

impl OpaqueRefreshTokenStore for InMemorySessionRevocationStore {
    fn store_refresh_token_record<'a>(
        &'a self,
        token_id: &'a str,
        record: &'a RefreshTokenRecord,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut records_guard = self.refresh_token_records.lock().unwrap();
            records_guard.insert(token_id.to_string(), record.clone());
            drop(records_guard);

            let mut session_tokens_guard = self.session_refresh_tokens.lock().unwrap();
            session_tokens_guard
                .entry(record.session_id.clone())
                .or_default()
                .insert(token_id.to_string());
            drop(session_tokens_guard);
            Ok(())
        })
    }

    fn get_refresh_token_record<'a>(
        &'a self,
        token_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<RefreshTokenRecord>>> {
        boxed(async move {
            let guard = self.refresh_token_records.lock().unwrap();
            Ok(guard.get(token_id).cloned())
        })
    }

    fn delete_refresh_token_record<'a>(
        &'a self,
        token_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
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
        })
    }

    fn delete_refresh_tokens_for_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>> {
        boxed(async move {
            let mut session_tokens_guard = self.session_refresh_tokens.lock().unwrap();
            if let Some(token_ids) = session_tokens_guard.remove(session_id) {
                drop(session_tokens_guard);
                let mut records_guard = self.refresh_token_records.lock().unwrap();
                for token_id in token_ids {
                    records_guard.remove(&token_id);
                }
            }

            Ok(())
        })
    }
}

#[must_use]
pub fn into_arc(store: InMemorySessionRevocationStore) -> Arc<dyn Store> {
    Arc::new(store)
}
