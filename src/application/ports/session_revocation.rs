use crate::application::ApplicationResult;
use async_trait::async_trait;

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
}
