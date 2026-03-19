use crate::application::ApplicationResult;

pub trait RefreshTokenCodec: Send + Sync {
    fn is_opaque_token(&self, token: &str) -> bool;
    fn encode_opaque_handle(&self, token_id: &str) -> ApplicationResult<String>;
    fn decode_opaque_handle(&self, token: &str) -> ApplicationResult<String>;
}
