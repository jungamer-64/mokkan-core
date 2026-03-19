use crate::application::{
    ApplicationResult,
    error::ApplicationError,
    ports::refresh_token::{RefreshTokenClaims, RefreshTokenCodec},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const TOKEN_PREFIX: &str = "rt2";

#[derive(Clone)]
pub struct HmacRefreshTokenCodec {
    secret: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncodedRefreshTokenClaims {
    sid: String,
    nonce: String,
    ver: u32,
}

impl HmacRefreshTokenCodec {
    pub fn new(secret: &str) -> ApplicationResult<Self> {
        if secret.is_empty() {
            return Err(ApplicationError::infrastructure(
                "refresh token secret must not be empty",
            ));
        }

        Ok(Self {
            secret: secret.as_bytes().to_vec(),
        })
    }

    fn sign(&self, payload: &[u8]) -> ApplicationResult<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| ApplicationError::infrastructure("invalid refresh token secret"))?;
        mac.update(payload);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

impl RefreshTokenCodec for HmacRefreshTokenCodec {
    fn can_decode(&self, token: &str) -> bool {
        token.starts_with(TOKEN_PREFIX) && token[TOKEN_PREFIX.len()..].starts_with('.')
    }

    fn encode(&self, claims: &RefreshTokenClaims) -> ApplicationResult<String> {
        let payload = EncodedRefreshTokenClaims {
            sid: claims.session_id.clone(),
            nonce: claims.nonce.clone(),
            ver: claims.token_version,
        };
        let payload_json = serde_json::to_vec(&payload)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let signature = self.sign(&payload_json)?;

        Ok(format!(
            "{TOKEN_PREFIX}.{}.{}",
            URL_SAFE_NO_PAD.encode(payload_json),
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    fn decode(&self, token: &str) -> ApplicationResult<RefreshTokenClaims> {
        let mut parts = token.split('.');
        let prefix = parts.next();
        let payload = parts.next();
        let signature = parts.next();

        if prefix != Some(TOKEN_PREFIX)
            || payload.is_none()
            || signature.is_none()
            || parts.next().is_some()
        {
            return Err(ApplicationError::validation("invalid refresh token"));
        }

        let payload = URL_SAFE_NO_PAD
            .decode(payload.unwrap().as_bytes())
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature.unwrap().as_bytes())
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| ApplicationError::infrastructure("invalid refresh token secret"))?;
        mac.update(&payload);
        mac.verify_slice(&signature)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        let claims: EncodedRefreshTokenClaims = serde_json::from_slice(&payload)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        Ok(RefreshTokenClaims {
            session_id: claims.sid,
            nonce: claims.nonce,
            token_version: claims.ver,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::HmacRefreshTokenCodec;
    use crate::application::ports::refresh_token::{RefreshTokenClaims, RefreshTokenCodec};

    #[test]
    fn refresh_token_codec_roundtrips_claims() {
        let codec = HmacRefreshTokenCodec::new("test-secret").unwrap();
        let token = codec
            .encode(&RefreshTokenClaims {
                session_id: "sid-1".into(),
                nonce: "nonce-1".into(),
                token_version: 3,
            })
            .unwrap();

        let decoded = codec.decode(&token).unwrap();
        assert_eq!(
            decoded,
            RefreshTokenClaims {
                session_id: "sid-1".into(),
                nonce: "nonce-1".into(),
                token_version: 3,
            }
        );
    }

    #[test]
    fn refresh_token_codec_rejects_tampering() {
        let codec = HmacRefreshTokenCodec::new("test-secret").unwrap();
        let token = codec
            .encode(&RefreshTokenClaims {
                session_id: "sid-1".into(),
                nonce: "nonce-1".into(),
                token_version: 3,
            })
            .unwrap();
        let tampered = format!("{token}x");

        assert!(codec.decode(&tampered).is_err());
    }
}
