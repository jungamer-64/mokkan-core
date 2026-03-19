use crate::application::{
    ApplicationResult, error::ApplicationError, ports::refresh_token::RefreshTokenCodec,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const OPAQUE_TOKEN_PREFIX: &str = "rt3";

#[derive(Clone)]
pub struct HmacRefreshTokenCodec {
    secret: Vec<u8>,
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

    fn verify_signature(&self, payload: &[u8], signature: &[u8]) -> ApplicationResult<()> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| ApplicationError::infrastructure("invalid refresh token secret"))?;
        mac.update(payload);
        mac.verify_slice(signature)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        Ok(())
    }

    fn parse_parts<'a>(&self, token: &'a str) -> ApplicationResult<(&'a str, &'a str)> {
        let mut parts = token.split('.');
        let prefix = parts.next();
        let token_id = parts.next();
        let signature = parts.next();

        if prefix != Some(OPAQUE_TOKEN_PREFIX)
            || token_id.is_none()
            || signature.is_none()
            || parts.next().is_some()
        {
            return Err(ApplicationError::validation("invalid refresh token"));
        }

        Ok((token_id.unwrap(), signature.unwrap()))
    }
}

impl RefreshTokenCodec for HmacRefreshTokenCodec {
    fn is_opaque_token(&self, token: &str) -> bool {
        token.starts_with(OPAQUE_TOKEN_PREFIX)
            && token[OPAQUE_TOKEN_PREFIX.len()..].starts_with('.')
    }

    fn encode_opaque_handle(&self, token_id: &str) -> ApplicationResult<String> {
        if token_id.is_empty() {
            return Err(ApplicationError::validation("invalid refresh token"));
        }

        let signature = self.sign(token_id.as_bytes())?;
        Ok(format!(
            "{OPAQUE_TOKEN_PREFIX}.{token_id}.{}",
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    fn decode_opaque_handle(&self, token: &str) -> ApplicationResult<String> {
        let (token_id, signature) = self.parse_parts(token)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature.as_bytes())
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        self.verify_signature(token_id.as_bytes(), &signature)?;
        Ok(token_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::HmacRefreshTokenCodec;
    use crate::application::ports::refresh_token::RefreshTokenCodec;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    #[test]
    fn refresh_token_codec_roundtrips_opaque_handles() {
        let codec = HmacRefreshTokenCodec::new("test-secret").unwrap();
        let token = codec.encode_opaque_handle("token-123").unwrap();

        let decoded = codec.decode_opaque_handle(&token).unwrap();
        assert_eq!(decoded, "token-123");
    }

    #[test]
    fn refresh_token_codec_rejects_tampering() {
        let codec = HmacRefreshTokenCodec::new("test-secret").unwrap();
        let token = codec.encode_opaque_handle("token-123").unwrap();
        let tampered = format!("{token}x");

        assert!(codec.decode_opaque_handle(&tampered).is_err());
    }

    #[test]
    fn refresh_token_codec_rejects_removed_rt2_tokens() {
        let codec = HmacRefreshTokenCodec::new("test-secret").unwrap();
        let payload = serde_json::to_vec(&serde_json::json!({
            "sid": "sid-1",
            "nonce": "nonce-1",
            "ver": 3
        }))
        .unwrap();
        let signature = codec.sign(&payload).unwrap();
        let removed_rt2 = format!(
            "rt2.{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(signature)
        );

        assert!(!codec.is_opaque_token(&removed_rt2));
        assert!(codec.decode_opaque_handle(&removed_rt2).is_err());
    }
}
