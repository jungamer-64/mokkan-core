use crate::application::ApplicationResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshTokenClaims {
    pub session_id: String,
    pub nonce: String,
    pub token_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedRefreshToken {
    OpaqueHandle { token_id: String },
    SignedClaims(RefreshTokenClaims),
}

pub trait RefreshTokenCodec: Send + Sync {
    fn can_decode(&self, token: &str) -> bool;
    fn encode_signed_claims(&self, claims: &RefreshTokenClaims) -> ApplicationResult<String>;
    fn encode_opaque_handle(&self, token_id: &str) -> ApplicationResult<String>;
    fn decode(&self, token: &str) -> ApplicationResult<DecodedRefreshToken>;
}
