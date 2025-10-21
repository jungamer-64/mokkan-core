// src/infrastructure/security/token.rs
use crate::application::{
    dto::{AuthTokenDto, AuthenticatedUser, TokenSubject},
    error::{ApplicationError, ApplicationResult},
    ports::security::TokenManager,
};
use crate::domain::user::Capability;
use async_trait::async_trait;
use biscuit_auth::{
    Biscuit, KeyPair, PrivateKey, PublicKey,
    builder::{Algorithm, AuthorizerBuilder, fact, string},
    builder_ext::AuthorizerExt,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
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

#[async_trait]
impl TokenManager for BiscuitTokenManager {
    async fn issue(&self, subject: TokenSubject) -> ApplicationResult<AuthTokenDto> {
        let issued_at = SystemTime::now();
        let expires_at = issued_at
            .checked_add(self.ttl)
            .ok_or_else(|| ApplicationError::infrastructure("token expiration overflow"))?;

        let mut params = HashMap::new();
        params.insert("uid".to_string(), (i64::from(subject.user_id)).into());
        params.insert("uname".to_string(), subject.username.clone().into());
        params.insert("urole".to_string(), subject.role.as_str().into());
        params.insert("issued".to_string(), issued_at.into());
        params.insert("exp".to_string(), expires_at.into());

        let mut builder = Biscuit::builder()
            .code_with_params(
                r#"
                user({uid}, {uname});
                role({urole});
                issued_at({issued});
                expires_at({exp});
                check if time($now), $now >= {issued};
                check if time($now), $now <= {exp};
                "#,
                params,
                HashMap::new(),
            )
            .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;

        for capability in subject.capabilities.iter() {
            builder = builder
                .fact(fact(
                    "right",
                    &[string(&capability.resource), string(&capability.action)],
                ))
                .map_err(|err| ApplicationError::infrastructure(err.to_string()))?;
        }

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

        parse_claims(facts)
    }
}

fn parse_claims(facts: Vec<biscuit_auth::builder::Fact>) -> ApplicationResult<AuthenticatedUser> {
    let mut user_id: Option<i64> = None;
    let mut username: Option<String> = None;
    let mut role: Option<crate::domain::user::Role> = None;
    let mut issued_at: Option<SystemTime> = None;
    let mut expires_at: Option<SystemTime> = None;
    let mut capabilities = std::collections::HashSet::new();

    for fact in facts {
        let predicate = fact.predicate;
        match predicate.name.as_str() {
            "user" => {
                if predicate.terms.len() == 2 {
                    if let biscuit_auth::builder::Term::Integer(id) = predicate.terms[0] {
                        user_id = Some(id);
                    }
                    if let biscuit_auth::builder::Term::Str(name) = predicate.terms[1].clone() {
                        username = Some(name);
                    }
                }
            }
            "role" => {
                if let Some(term) = predicate.terms.first() {
                    if let biscuit_auth::builder::Term::Str(role_name) = term.clone() {
                        role = Some(role_name.parse().map_err(ApplicationError::from)?);
                    }
                }
            }
            "issued_at" => {
                if let Some(term) = predicate.terms.first() {
                    if let biscuit_auth::builder::Term::Date(seconds) = term {
                        issued_at = Some(UNIX_EPOCH + std::time::Duration::from_secs(*seconds));
                    }
                }
            }
            "expires_at" => {
                if let Some(term) = predicate.terms.first() {
                    if let biscuit_auth::builder::Term::Date(seconds) = term {
                        expires_at = Some(UNIX_EPOCH + std::time::Duration::from_secs(*seconds));
                    }
                }
            }
            "right" => {
                if predicate.terms.len() == 2 {
                    if let (
                        biscuit_auth::builder::Term::Str(resource),
                        biscuit_auth::builder::Term::Str(action),
                    ) = (predicate.terms[0].clone(), predicate.terms[1].clone())
                    {
                        capabilities.insert(Capability::new(resource, action));
                    }
                }
            }
            _ => {}
        }
    }

    let user_id = user_id.ok_or_else(|| ApplicationError::unauthorized("missing user id"))?;
    let username = username.ok_or_else(|| ApplicationError::unauthorized("missing username"))?;
    let role = role.ok_or_else(|| ApplicationError::unauthorized("missing role"))?;
    let issued_at = issued_at.ok_or_else(|| ApplicationError::unauthorized("missing issued_at"))?;
    let expires_at =
        expires_at.ok_or_else(|| ApplicationError::unauthorized("missing expires_at"))?;

    let user_id =
        crate::domain::user::UserId::new(user_id).map_err(|err| ApplicationError::from(err))?;

    let mut all_caps = role.default_capabilities();
    all_caps.extend(capabilities);

    Ok(AuthenticatedUser {
        id: user_id,
        username,
        role,
        capabilities: all_caps,
        issued_at: DateTime::<Utc>::from(issued_at),
        expires_at: DateTime::<Utc>::from(expires_at),
    })
}
