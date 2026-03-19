use crate::application::ApplicationResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshTokenClaims {
    pub session_id: String,
    pub nonce: String,
    pub token_version: u32,
}

pub trait RefreshTokenCodec: Send + Sync {
    fn can_decode(&self, token: &str) -> bool;
    fn encode(&self, claims: &RefreshTokenClaims) -> ApplicationResult<String>;
    fn decode(&self, token: &str) -> ApplicationResult<RefreshTokenClaims>;
}
