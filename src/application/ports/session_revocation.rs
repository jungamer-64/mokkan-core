use crate::application::ApplicationResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Information about a session stored in the backing store.
/// `created_at_unix` is seconds since epoch (UTC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user_id: i64,
    pub session_id: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at_unix: i64,
    pub revoked: bool,
}

#[async_trait]
pub trait SessionRevocationStore: Send + Sync {
    /// Return true if the given session id has been revoked.
    async fn is_revoked(&self, session_id: &str) -> ApplicationResult<bool>;

    /// Revoke the given session id (e.g. on logout).
    async fn revoke(&self, session_id: &str) -> ApplicationResult<()>;

    /// Get the minimum allowed token version for a user. Tokens with a version less
    /// than this should be considered invalid.
    async fn get_min_token_version(&self, user_id: i64) -> ApplicationResult<Option<u32>>;

    /// Set the minimum allowed token version for a user.
    async fn set_min_token_version(&self, user_id: i64, min_version: u32) -> ApplicationResult<()>;

    /// Store the current refresh nonce for a session (used for refresh-token rotation).
    async fn set_session_refresh_nonce(&self, session_id: &str, nonce: &str) -> ApplicationResult<()>;

    /// Get the current refresh nonce for a session.
    async fn get_session_refresh_nonce(&self, session_id: &str) -> ApplicationResult<Option<String>>;

    /// Atomically compare-and-swap the session's refresh nonce.
    ///
    /// If the currently stored nonce for `session_id` equals `expected`, it will be
    /// replaced with `new_nonce` and the method returns Ok(true). If the current value
    /// does not match `expected`, the store is left unchanged and Ok(false) is returned.
    async fn compare_and_swap_session_refresh_nonce(
        &self,
        session_id: &str,
        expected: &str,
        new_nonce: &str,
    ) -> ApplicationResult<bool>;

    /// Mark a specific refresh nonce for a session as used (so that later reuse can be detected).
    async fn mark_session_refresh_nonce_used(&self, session_id: &str, nonce: &str) -> ApplicationResult<()>;

    /// Return true if the given nonce for the session has been used before.
    async fn is_session_refresh_nonce_used(&self, session_id: &str, nonce: &str) -> ApplicationResult<bool>;

    /// Track that a session id belongs to a user (used for per-user session listing and bulk revocation).
    async fn add_session_for_user(&self, user_id: i64, session_id: &str) -> ApplicationResult<()>;

    /// Remove the association of a session id from a user.
    async fn remove_session_for_user(&self, user_id: i64, session_id: &str) -> ApplicationResult<()>;

    /// List session ids for a given user.
    async fn list_sessions_for_user(&self, user_id: i64) -> ApplicationResult<Vec<String>>;

    /// List sessions for a given user including stored metadata (user agent, ip, created_at, revoked).
    async fn list_sessions_for_user_with_meta(&self, user_id: i64) -> ApplicationResult<Vec<SessionInfo>>;

    /// Store or update session metadata. `created_at_unix` is seconds since epoch UTC.
    async fn set_session_metadata(
        &self,
        user_id: i64,
        session_id: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        created_at_unix: i64,
    ) -> ApplicationResult<()>;

    /// Get session metadata for a given session id.
    async fn get_session_metadata(&self, session_id: &str) -> ApplicationResult<Option<SessionInfo>>;

    /// Delete session metadata (e.g. when a session is removed from the user's list).
    async fn delete_session_metadata(&self, session_id: &str) -> ApplicationResult<()>;

    /// Revoke all sessions for a given user (used when refresh reuse is detected).
    async fn revoke_sessions_for_user(&self, user_id: i64) -> ApplicationResult<()>;
}
