use crate::application::AppResult;
use crate::async_support::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshTokenRecord {
    pub session_id: String,
    pub nonce: String,
    pub token_version: u32,
}

pub trait Revocation: Send + Sync {
    /// Return true if the given session id has been revoked.
    fn is_revoked<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<bool>>;

    /// Revoke the given session id (e.g. on logout).
    fn revoke<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<()>>;

    /// Revoke all sessions for a given user (used when refresh reuse is detected).
    fn revoke_sessions_for_user(&self, user_id: i64) -> BoxFuture<'_, AppResult<()>>;
}

pub trait TokenVersionStore: Send + Sync {
    /// Get the minimum allowed token version for a user. Tokens with a version less
    /// than this should be considered invalid.
    fn get_min_token_version(&self, user_id: i64) -> BoxFuture<'_, AppResult<Option<u32>>>;

    /// Set the minimum allowed token version for a user.
    fn set_min_token_version(&self, user_id: i64, min_version: u32)
    -> BoxFuture<'_, AppResult<()>>;
}

pub trait RefreshNonceStore: Send + Sync {
    /// Store the current refresh nonce for a session (used for refresh-token rotation).
    fn set_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// Get the current refresh nonce for a session.
    fn get_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<String>>>;

    /// Atomically compare-and-swap the session's refresh nonce.
    ///
    /// If the currently stored nonce for `session_id` equals `expected`, it will be
    /// replaced with `new_nonce` and the method returns Ok(true). If the current value
    /// does not match `expected`, the store is left unchanged and Ok(false) is returned.
    fn compare_and_swap_session_refresh_nonce<'a>(
        &'a self,
        session_id: &'a str,
        expected: &'a str,
        new_nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<bool>>;

    /// Mark a specific refresh nonce for a session as used (so that later reuse can be detected).
    fn mark_session_refresh_nonce_used<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// Return true if the given nonce for the session has been used before.
    fn is_session_refresh_nonce_used<'a>(
        &'a self,
        session_id: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, AppResult<bool>>;
}

pub trait SessionMetadataStore: Send + Sync {
    /// Track that a session id belongs to a user (used for per-user session listing and bulk revocation).
    fn add_session_for_user<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// Remove the association of a session id from a user.
    fn remove_session_for_user<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// List session ids for a given user.
    fn list_sessions_for_user(&self, user_id: i64) -> BoxFuture<'_, AppResult<Vec<String>>>;

    /// List sessions for a given user including stored metadata.
    ///
    /// The returned entries include user agent, `IP` address, `created_at`,
    /// and revocation state.
    fn list_sessions_for_user_with_meta(
        &self,
        user_id: i64,
    ) -> BoxFuture<'_, AppResult<Vec<SessionInfo>>>;

    /// Store or update session metadata. `created_at_unix` is seconds since epoch UTC.
    fn set_session_metadata<'a>(
        &'a self,
        user_id: i64,
        session_id: &'a str,
        user_agent: Option<&'a str>,
        ip_address: Option<&'a str>,
        created_at_unix: i64,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// Get session metadata for a given session id.
    fn get_session_metadata<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<SessionInfo>>>;

    /// Delete session metadata (e.g. when a session is removed from the user's list).
    fn delete_session_metadata<'a>(&'a self, session_id: &'a str) -> BoxFuture<'a, AppResult<()>>;
}

pub trait OpaqueRefreshTokenStore: Send + Sync {
    /// Store the server-side record for an opaque refresh token handle.
    fn store_refresh_token_record<'a>(
        &'a self,
        token_id: &'a str,
        record: &'a RefreshTokenRecord,
    ) -> BoxFuture<'a, AppResult<()>>;

    /// Load the stored record for an opaque refresh token handle.
    fn get_refresh_token_record<'a>(
        &'a self,
        token_id: &'a str,
    ) -> BoxFuture<'a, AppResult<Option<RefreshTokenRecord>>>;

    /// Delete a single opaque refresh token handle.
    fn delete_refresh_token_record<'a>(&'a self, token_id: &'a str)
    -> BoxFuture<'a, AppResult<()>>;

    /// Delete every opaque refresh token handle associated with a session.
    fn delete_refresh_tokens_for_session<'a>(
        &'a self,
        session_id: &'a str,
    ) -> BoxFuture<'a, AppResult<()>>;
}

pub trait Store:
    Revocation
    + TokenVersionStore
    + RefreshNonceStore
    + SessionMetadataStore
    + OpaqueRefreshTokenStore
    + Send
    + Sync
{
}

impl<T> Store for T where
    T: Revocation
        + TokenVersionStore
        + RefreshNonceStore
        + SessionMetadataStore
        + OpaqueRefreshTokenStore
        + Send
        + Sync
{
}

#[derive(Clone)]
#[must_use]
pub struct Ports {
    pub revocation: Arc<dyn Revocation>,
    pub token_versions: Arc<dyn TokenVersionStore>,
    pub refresh_nonces: Arc<dyn RefreshNonceStore>,
    pub session_metadata: Arc<dyn SessionMetadataStore>,
    pub opaque_refresh_tokens: Arc<dyn OpaqueRefreshTokenStore>,
}

impl Ports {
    pub fn from_store(store: Arc<dyn Store>) -> Self {
        Self {
            revocation: store.clone(),
            token_versions: store.clone(),
            refresh_nonces: store.clone(),
            session_metadata: store.clone(),
            opaque_refresh_tokens: store,
        }
    }
}
