// src/infrastructure/security/token.rs
use crate::application::{
    dto::{AuthTokenDto, AuthenticatedUser, TokenSubject},
    error::{ApplicationError, ApplicationResult},
    ports::security::TokenManager,
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use biscuit_auth::{
    Biscuit, KeyPair, PrivateKey, PublicKey,
    builder::{Algorithm, BlockBuilder, Term},
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
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
    issued_at: SystemTime,
    expires_at: SystemTime,
) -> (String, HashMap<String, Term>) {
    let mut params: HashMap<String, Term> = HashMap::new();
    params.insert("uid".to_string(), (i64::from(subject.user_id)).into());
    params.insert("uname".to_string(), subject.username.clone().into());
    params.insert("urole".to_string(), subject.role.as_str().into());
    params.insert("issued".to_string(), issued_at.into());
    params.insert("exp".to_string(), expires_at.into());

    let mut code = String::from(
        r#"
                user({uid}, {uname});
                role({urole});
                issued_at({issued});
                expires_at({exp});
                check if time($now), $now >= {issued};
                check if time($now), $now <= {exp};
                "#,
    );

    if let Some(sid) = subject.session_id.as_ref() {
        code.push_str("session({sid}, {ver});\n");
        params.insert("sid".to_string(), sid.clone().into());
        let ver = subject.token_version.unwrap_or(1) as i64;
        params.insert("ver".to_string(), ver.into());
    }

    // Include token_type as a root fact so caveat checks can validate against it.
    // Default to "access" for issued tokens from the manager.
    params.insert("tt".to_string(), "access".to_string().into());
    code.push_str("token_type({tt});\n");

    // Append capability facts into the code using parameters to avoid manual escaping.
    for (i, cap) in subject.capabilities.iter().enumerate() {
        let res_key = format!("cap_res_{}", i);
        let act_key = format!("cap_act_{}", i);
        params.insert(res_key.clone(), cap.resource.clone().into());
        params.insert(act_key.clone(), cap.action.clone().into());
        code.push_str(&format!("right({{{}}}, {{{}}});\n", res_key, act_key));
    }

    (code, params)
}

fn build_caveat_code_and_params(token_type: &str) -> (String, HashMap<String, Term>) {
    let mut params: HashMap<String, Term> = HashMap::new();
    params.insert("tt".to_string(), token_type.to_string().into());

    // Caveat block: include a marker fact to indicate a caveat block is present
    // and require the token_type via a check. The actual token_type fact is
    // provided by the root block (authority), so the check will validate it.
    // Use a 1-arity marker fact to avoid parser issues with zero-arity predicates.
    let code = String::from(
        r#"
                has_caveat("1");
                check if token_type({tt});
                "#,
    );

    (code, params)
}

fn seal_and_serialize(token: Biscuit) -> Result<String, ApplicationError> {
    let sealed = token
        .seal()
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
    sealed
        .to_base64()
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))
}

fn ttl_to_expires_in_seconds(ttl: Duration) -> i64 {
    ChronoDuration::from_std(ttl)
        .unwrap_or_else(|_| ChronoDuration::seconds(ttl.as_secs() as i64))
        .num_seconds()
        .max(0)
}

#[allow(dead_code)]
fn build_and_serialize_biscuit(
    code: &str,
    params: HashMap<String, Term>,
    root: &KeyPair,
) -> Result<String, ApplicationError> {
    let builder = Biscuit::builder()
        .code_with_params(code, params, HashMap::new())
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

    let token = builder
        .build(root)
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

    seal_and_serialize(token)
}

fn build_and_serialize_biscuit_with_block(
    root_code: &str,
    root_params: HashMap<String, Term>,
    block_code: &str,
    block_params: HashMap<String, Term>,
    root: &KeyPair,
) -> Result<String, ApplicationError> {
    let builder = Biscuit::builder()
        .code_with_params(root_code, root_params, HashMap::new())
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

    let block = BlockBuilder::new()
        .code_with_params(block_code, block_params, HashMap::new())
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

    let builder = builder.merge(block);

    let token = builder
        .build(root)
        .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

    seal_and_serialize(token)
}

fn extract_root_token_type_from_facts(facts: &Vec<biscuit_auth::builder::Fact>) -> Option<String> {
    for f in facts.iter() {
        if f.predicate.name == "token_type" {
            if let Some(term) = f.predicate.terms.first() {
                if let Term::Str(s) = term.clone() {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn ensure_checks_match_root_tt(
    checks: &Vec<biscuit_auth::builder::Check>,
    root_tt: &str,
) -> Result<(), ApplicationError> {
    for check in checks.iter() {
        for rule in check.queries.iter() {
            for pred in rule.body.iter() {
                if pred.name == "token_type" {
                    if let Some(term) = pred.terms.first() {
                        match term.clone() {
                            Term::Str(ref s) => {
                                if s != root_tt {
                                    return Err(ApplicationError::unauthorized(
                                        "token caveat does not match authority token_type",
                                    ));
                                }
                            }
                            _ => {
                                return Err(ApplicationError::unauthorized(
                                    "invalid token_type term in caveat",
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[async_trait]
impl TokenManager for BiscuitTokenManager {
    async fn issue(&self, subject: TokenSubject) -> ApplicationResult<AuthTokenDto> {
        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(self.ttl)
            .ok_or_else(|| ApplicationError::infrastructure("token expiration overflow"))?;
        let (code, params) = build_code_and_params(&subject, issued_at, expires_at);

        // Build a separate caveat block for token_type and merge it into the biscuit.
        let (caveat_code, caveat_params) = build_caveat_code_and_params("access");
        let serialized = build_and_serialize_biscuit_with_block(
            &code,
            params,
            &caveat_code,
            caveat_params,
            self.root.as_ref(),
        )?;

        let issued_at_dt = DateTime::<Utc>::from(issued_at);
        let expires_at_dt = DateTime::<Utc>::from(expires_at);
        let expires_in = ttl_to_expires_in_seconds(self.ttl);

        Ok(AuthTokenDto {
            token: serialized,
            issued_at: issued_at_dt,
            expires_at: expires_at_dt,
            expires_in,
            session_id: subject.session_id.clone(),
            refresh_token: None,
        })
    }

    async fn public_jwk(&self) -> ApplicationResult<serde_json::Value> {
        // For Ed25519 (OKP) produce a minimal JWK with x parameter (base64url)
        let key_bytes = self.public.to_bytes();
        let x = URL_SAFE_NO_PAD.encode(key_bytes);

        // Compute JWK thumbprint (RFC 7638) for a stable `kid` value.
        // For OKP/Ed25519, the canonical members are {"crv":"Ed25519","kty":"OKP","x":"<x>"}
        let thumbprint_input = format!(r#"{{"crv":"Ed25519","kty":"OKP","x":"{}"}}"#, x);
        let mut hasher = Sha256::new();
        hasher.update(thumbprint_input.as_bytes());
        let kid = URL_SAFE_NO_PAD.encode(hasher.finalize());

        let jwk = json!({
            "keys": [
                {
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "EdDSA",
                    "use": "sig",
                    "x": x,
                    "kid": kid,
                }
            ]
        });

        Ok(jwk)
    }

    async fn authenticate(&self, token: &str) -> ApplicationResult<AuthenticatedUser> {
        let biscuit = Biscuit::from_base64(token, self.public)
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;

        // Inspect the biscuit view before authorizing so we can surface meaningful
        // debug information when checks fail.
        let view = biscuit
            .authorizer()
            .map_err(|err| ApplicationError::unauthorized(err.to_string()))?;

        // Dump facts and checks so we can enforce that the caveat block's
        // `check if token_type({tt})` actually matches the root `token_type` fact.
        let (facts, _rules, checks, _policies) = view.dump();

        // Enforce presence of our caveat marker; tokens without the caveat
        // block should be considered unauthorized.
        let has_caveat = facts.iter().any(|f| f.predicate.name == "has_caveat");
        if !has_caveat {
            return Err(ApplicationError::unauthorized(
                "missing required token caveat",
            ));
        }

        let root_tt = extract_root_token_type_from_facts(&facts)
            .ok_or_else(|| ApplicationError::unauthorized("missing token_type"))?;

        ensure_checks_match_root_tt(&checks, &root_tt)?;

        // Parse claims into an AuthenticatedUser and perform simple time checks
        // (issued_at <= now <= expires_at).
        let user = crate::infrastructure::security::claims::parse_claims(facts)?;
        let now = chrono::Utc::now();
        if now < user.issued_at || now > user.expires_at {
            return Err(ApplicationError::unauthorized(
                "token is expired or not yet valid",
            ));
        }

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{Capability, Role, UserId};
    use std::collections::HashSet;
    use std::time::{Duration as StdDuration, SystemTime};

    #[tokio::test]
    async fn authenticate_rejects_token_without_caveat() {
        // Build a deterministic keypair from a known hex (matches .env sample used in the repo)
        let private_hex = "6937d945f8dbe222ae559a9d341a9c70071ef4565367dcf02bf7d5b03a46df1f";
        let private = PrivateKey::from_bytes_hex(private_hex, Algorithm::Ed25519)
            .expect("create private key");
        let keypair = KeyPair::from(&private);
        let public = keypair.public();
        let root = Arc::new(keypair);

        let manager = BiscuitTokenManager {
            root: root.clone(),
            public,
            ttl: StdDuration::from_secs(3600),
        };

        // Create a simple subject
        let mut caps = HashSet::new();
        caps.insert(Capability::new("articles", "create"));

        let subject = TokenSubject {
            user_id: UserId::new(1).unwrap(),
            username: "alice".to_string(),
            role: Role::Author,
            capabilities: caps,
            session_id: None,
            token_version: None,
        };

        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(StdDuration::from_secs(3600))
            .expect("overflow");

        // Build a biscuit WITHOUT the separate caveat block
        let (code, params) = build_code_and_params(&subject, issued_at, expires_at);
        let token =
            build_and_serialize_biscuit(&code, params, manager.root.as_ref()).expect("build token");

        let res = manager.authenticate(&token).await;
        assert!(
            res.is_err(),
            "expected authentication to fail for token without caveat"
        );
    }

    #[tokio::test]
    async fn authenticate_accepts_token_with_access_caveat() {
        let private_hex = "6937d945f8dbe222ae559a9d341a9c70071ef4565367dcf02bf7d5b03a46df1f";
        let private = PrivateKey::from_bytes_hex(private_hex, Algorithm::Ed25519)
            .expect("create private key");
        let keypair = KeyPair::from(&private);
        let public = keypair.public();
        let root = Arc::new(keypair);

        let manager = BiscuitTokenManager {
            root: root.clone(),
            public,
            ttl: StdDuration::from_secs(3600),
        };

        let mut caps = HashSet::new();
        caps.insert(Capability::new("articles", "create"));

        let subject = TokenSubject {
            user_id: UserId::new(1).unwrap(),
            username: "alice".to_string(),
            role: Role::Author,
            capabilities: caps,
            session_id: None,
            token_version: None,
        };

        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(StdDuration::from_secs(3600))
            .expect("overflow");

        // Build a biscuit WITH the separate caveat block for token_type("access")
        let (code, params) = build_code_and_params(&subject, issued_at, expires_at);
        let (caveat_code, caveat_params) = build_caveat_code_and_params("access");
        let token = build_and_serialize_biscuit_with_block(
            &code,
            params,
            &caveat_code,
            caveat_params,
            manager.root.as_ref(),
        )
        .expect("build token with block");

        let res = manager.authenticate(&token).await;
        if let Err(e) = &res {
            eprintln!("authenticate error: {:?}", e);
        }
        assert!(
            res.is_ok(),
            "expected authentication to succeed for token with access caveat"
        );
    }

    #[tokio::test]
    async fn authenticate_rejects_token_with_wrong_caveat() {
        let private_hex = "6937d945f8dbe222ae559a9d341a9c70071ef4565367dcf02bf7d5b03a46df1f";
        let private = PrivateKey::from_bytes_hex(private_hex, Algorithm::Ed25519)
            .expect("create private key");
        let keypair = KeyPair::from(&private);
        let public = keypair.public();
        let root = Arc::new(keypair);

        let manager = BiscuitTokenManager {
            root: root.clone(),
            public,
            ttl: StdDuration::from_secs(3600),
        };

        let mut caps = HashSet::new();
        caps.insert(Capability::new("articles", "create"));

        let subject = TokenSubject {
            user_id: UserId::new(1).unwrap(),
            username: "alice".to_string(),
            role: Role::Author,
            capabilities: caps,
            session_id: None,
            token_version: None,
        };

        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(StdDuration::from_secs(3600))
            .expect("overflow");

        // Build a biscuit WITH a caveat block that expects token_type("refresh")
        // while the root token_type is "access". This should be rejected.
        let (code, params) = build_code_and_params(&subject, issued_at, expires_at);
        let (caveat_code, caveat_params) = build_caveat_code_and_params("refresh");
        let token = build_and_serialize_biscuit_with_block(
            &code,
            params,
            &caveat_code,
            caveat_params,
            manager.root.as_ref(),
        )
        .expect("build token with bad caveat");

        let res = manager.authenticate(&token).await;
        assert!(
            res.is_err(),
            "expected authentication to fail for token with mismatched caveat"
        );
    }
}
