// src/infrastructure/security/token.rs
use crate::application::{
    dto::{AuthTokenDto, AuthenticatedUser, TokenSubject},
    error::{ApplicationError, ApplicationResult},
    ports::security::TokenManager,
};
use async_trait::async_trait;
use biscuit_auth::{
    Biscuit, KeyPair, PrivateKey, PublicKey,
    builder::{Algorithm, AuthorizerBuilder, fact, string},
    builder_ext::AuthorizerExt,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::json;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub struct BiscuitTokenManager {
    root: Arc<KeyPair>,
    public: PublicKey,
    ttl: Duration,
}

impl BiscuitTokenManager {
    pub fn new(private_key_hex: &str, ttl: Duration) -> ApplicationResult<Self> {
        let private = PrivateKey::from_bytes_hex(private_key_hex, Algorithm::Ed25519)
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        let keypair = KeyPair::from(&private);
        let public = keypair.public();

        Ok(Self {
            root: Arc::new(keypair),
            public,
            ttl,
        })
    }
}

fn build_code_and_params(
    subject: &TokenSubject,
    issued_at: std::time::SystemTime,
    expires_at: std::time::SystemTime,
) -> (String, std::collections::HashMap<String, biscuit_auth::builder::Term>) {
    let mut params: std::collections::HashMap<String, biscuit_auth::builder::Term> =
        std::collections::HashMap::new();
    params.insert("uid".to_string(), (i64::from(subject.user_id)).into());
    params.insert("uname".to_string(), subject.username.clone().into());
    params.insert("urole".to_string(), subject.role.as_str().into());
    params.insert("issued".to_string(), issued_at.into());
    params.insert("exp".to_string(), expires_at.into());

    let mut code = String::from(r#"
                user({uid}, {uname});
                role({urole});
                issued_at({issued});
                expires_at({exp});
                check if time($now), $now >= {issued};
                check if time($now), $now <= {exp};
                token_type("access");
                "#);

    if let Some(sid) = subject.session_id.as_ref() {
        code.push_str("session({sid}, {ver});\n");
        params.insert("sid".to_string(), sid.clone().into());
        let ver = subject.token_version.unwrap_or(1) as i64;
        params.insert("ver".to_string(), ver.into());
    }

    (code, params)
}

#[async_trait]
impl TokenManager for BiscuitTokenManager {
    async fn issue(&self, subject: TokenSubject) -> ApplicationResult<AuthTokenDto> {
        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(self.ttl)
            .ok_or_else(|| ApplicationError::infrastructure("token expiration overflow"))?;
        let (code, params) = build_code_and_params(&subject, issued_at, expires_at);

        let builder = Biscuit::builder()
            .code_with_params(&code, params, HashMap::new())
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let builder = subject
            .capabilities
            .iter()
            .try_fold(builder, |b, capability| {
                b.fact(fact(
                    "right",
                    & [string(&capability.resource), string(&capability.action)][..],
                ))
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))
            })?;

        let token = builder
            .build(self.root.as_ref())
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        let sealed = token
            .seal()
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        let serialized = sealed
            .to_base64()
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        let issued_at_dt = DateTime::<Utc>::from(issued_at);
        let expires_at_dt = DateTime::<Utc>::from(expires_at);
        let expires_in = ChronoDuration::from_std(self.ttl)
            .unwrap_or_else(|_| ChronoDuration::seconds(self.ttl.as_secs() as i64))
            .num_seconds()
            .max(0);

        Ok(AuthTokenDto {
            token: serialized,
            issued_at: issued_at_dt,
            expires_at: expires_at_dt,
            expires_in,
        })
    }

    async fn public_jwk(&self) -> ApplicationResult<serde_json::Value> {
        // For Ed25519 (OKP) produce a minimal JWK with x parameter (base64url)
        let key_bytes = self.public.to_bytes();
        let x = URL_SAFE_NO_PAD.encode(key_bytes);

        let jwk = json!({
            "keys": [
                {
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "EdDSA",
                    "use": "sig",
                    "x": x,
                    "kid": self.public.to_bytes_hex(),
                }
            ]
        });

        Ok(jwk)
    }

    async fn authenticate(&self, token: &str) -> ApplicationResult<AuthenticatedUser> {
        let biscuit = Biscuit::from_base64(token, self.public)
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;

        let mut authorizer = AuthorizerBuilder::new()
            .time()
            .allow_all()
            .build(&biscuit)
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;

        authorizer
            .authorize()
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;

        let view = biscuit
            .authorizer()
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;
        let (facts, _, _, _) = view.dump();

        crate::infrastructure::security::claims::parse_claims(facts)
    }
}
