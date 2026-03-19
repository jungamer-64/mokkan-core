#![allow(clippy::module_name_repetitions)]

use crate::application::ApplicationResult;

pub trait RefreshTokenCodec: Send + Sync {
    #[must_use]
    fn is_opaque_token(&self, token: &str) -> bool;

    /// Encode an opaque refresh token handle for transport.
    ///
    /// # Errors
    ///
    /// Returns an error if the handle is invalid or signing fails.
    fn encode_opaque_handle(&self, token_id: &str) -> ApplicationResult<String>;

    /// Decode and verify an opaque refresh token handle.
    ///
    /// # Errors
    ///
    /// Returns an error if the token format or signature is invalid.
    fn decode_opaque_handle(&self, token: &str) -> ApplicationResult<String>;
}
