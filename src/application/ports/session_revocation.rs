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
}
