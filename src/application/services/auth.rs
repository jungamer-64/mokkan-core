use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Duration;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use crate::application::{
    AppError, AppResult, AuthTokenDto, AuthenticatedUser, TokenSubject,
    ports::{
        authorization_code::{Code, CodeStore},
        security::TokenManager,
        session_revocation::{Ports, Store},
        time::Clock,
    },
    random_id,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueAuthorizationCodeRequest {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueAuthorizationCodeResult {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeAuthorizationCodeRequest {
    pub code: String,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenIntrospection {
    pub active: bool,
    pub scope: Option<String>,
    pub username: Option<String>,
    pub sub: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub session_id: Option<String>,
}

impl TokenIntrospection {
    const fn inactive() -> Self {
        Self {
            active: false,
            scope: None,
            username: None,
            sub: None,
            exp: None,
            iat: None,
            session_id: None,
        }
    }

    fn active(user: AuthenticatedUser) -> Self {
        Self {
            active: true,
            scope: Some("openid profile email".into()),
            username: Some(user.username.clone()),
            sub: Some(i64::from(user.id).to_string()),
            exp: Some(user.expires_at.timestamp()),
            iat: Some(user.issued_at.timestamp()),
            session_id: user.session_id,
        }
    }
}

#[derive(Clone)]
pub struct AuthService {
    token_manager: Arc<dyn TokenManager>,
    session_stores: Ports,
    authorization_code_store: Arc<dyn CodeStore>,
    clock: Arc<dyn Clock>,
}

impl AuthService {
    #[must_use]
    pub fn new(
        token_manager: Arc<dyn TokenManager>,
        session_revocation_store: Arc<dyn Store>,
        authorization_code_store: Arc<dyn CodeStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            token_manager,
            session_stores: Ports::from_store(session_revocation_store),
            authorization_code_store,
            clock,
        }
    }

    /// Authenticate a raw token and enforce revocation rules.
    ///
    /// # Errors
    ///
    /// Returns an error if the token is invalid, revoked, or expired.
    pub async fn authenticate(&self, token: &str) -> AppResult<AuthenticatedUser> {
        let user = self.token_manager.authenticate(token).await?;
        self.ensure_session_not_revoked(&user).await?;
        self.ensure_token_version_not_revoked(&user).await?;
        Ok(user)
    }

    /// Authenticate a raw token and ensure the user has the requested capability.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication or authorization fails.
    pub async fn authenticate_and_authorize(
        &self,
        token: &str,
        resource: &str,
        action: &str,
    ) -> AppResult<AuthenticatedUser> {
        let user = self.authenticate(token).await?;
        Self::ensure_has_capability(&user, resource, action)?;
        Ok(user)
    }

    /// Return the public JWK representation for token verification.
    ///
    /// # Errors
    ///
    /// Returns an error if the token manager cannot render the public key.
    pub async fn public_jwk(&self) -> AppResult<JsonValue> {
        self.token_manager.public_jwk().await
    }

    /// Issue and persist an authorization code.
    ///
    /// # Errors
    ///
    /// Returns an error if the redirect URI is invalid, a code cannot be
    /// generated, or the code store fails.
    pub async fn issue_authorization_code(
        &self,
        user: &AuthenticatedUser,
        request: IssueAuthorizationCodeRequest,
    ) -> AppResult<IssueAuthorizationCodeResult> {
        Self::validate_authorize_redirect_uri(request.redirect_uri.as_deref())?;

        let code = random_id::v4_string()?;
        let now = self.clock.now();
        let auth_code = Code {
            code: code.clone(),
            client_id: request.client_id,
            redirect_uri: request.redirect_uri,
            subject: TokenSubject::from_authenticated(user),
            scope: request.scope,
            code_challenge: request.code_challenge,
            code_challenge_method: request.code_challenge_method,
            created_at: now,
            expires_at: now + Duration::minutes(5),
        };

        self.authorization_code_store.create_code(auth_code).await?;
        Ok(IssueAuthorizationCodeResult { code })
    }

    /// Exchange an authorization code for tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if the code is missing, expired, already consumed, or
    /// PKCE/redirect URI validation fails.
    pub async fn exchange_authorization_code(
        &self,
        request: ExchangeAuthorizationCodeRequest,
    ) -> AppResult<AuthTokenDto> {
        let stored = self
            .authorization_code_store
            .consume_code(&request.code)
            .await?
            .ok_or_else(|| AppError::validation("invalid or expired code"))?;

        Self::validate_exchange_redirect_uri(&stored, request.redirect_uri.as_deref())?;
        Self::verify_pkce(&stored, request.code_verifier.as_deref())?;

        self.token_manager.issue(stored.subject).await
    }

    /// Introspect a raw token without enforcing revocation state.
    ///
    /// Invalid tokens produce an inactive response rather than an error.
    ///
    /// # Errors
    ///
    /// Returns an error only if a successful introspection cannot be rendered.
    pub async fn introspect_token(&self, token: &str) -> AppResult<TokenIntrospection> {
        self.token_manager.authenticate(token).await.map_or_else(
            |_| Ok(TokenIntrospection::inactive()),
            |user| Ok(TokenIntrospection::active(user)),
        )
    }

    /// Revoke a token's session when the token is valid and session-backed.
    ///
    /// Invalid tokens are ignored to preserve endpoint semantics.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing session exists but cannot be revoked.
    pub async fn revoke_token(&self, token: &str) -> AppResult<()> {
        if let Ok(user) = self.token_manager.authenticate(token).await
            && let Some(session_id) = user.session_id.as_deref()
        {
            self.session_stores.revocation.revoke(session_id).await?;
        }

        Ok(())
    }

    /// Revoke the current authenticated session.
    ///
    /// # Errors
    ///
    /// Returns an error if the token is not session-based or revocation fails.
    pub async fn logout(&self, user: &AuthenticatedUser) -> AppResult<()> {
        if let Some(session_id) = user.session_id.as_deref() {
            self.session_stores.revocation.revoke(session_id).await
        } else {
            Err(AppError::validation("token is not session-based"))
        }
    }

    async fn ensure_session_not_revoked(&self, user: &AuthenticatedUser) -> AppResult<()> {
        if let Some(session_id) = &user.session_id
            && self
                .session_stores
                .revocation
                .is_revoked(session_id)
                .await?
        {
            return Err(AppError::unauthorized("session revoked"));
        }

        Ok(())
    }

    async fn ensure_token_version_not_revoked(&self, user: &AuthenticatedUser) -> AppResult<()> {
        if let Some(token_version) = user.token_version
            && let Some(min_version) = self
                .session_stores
                .token_versions
                .get_min_token_version(i64::from(user.id))
                .await?
            && token_version < min_version
        {
            return Err(AppError::unauthorized("token revoked"));
        }

        Ok(())
    }

    fn ensure_has_capability(
        user: &AuthenticatedUser,
        resource: &str,
        action: &str,
    ) -> AppResult<()> {
        if user.has_capability(resource, action) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "missing capability {resource}:{action}"
            )))
        }
    }

    fn validate_authorize_redirect_uri(redirect_uri: Option<&str>) -> AppResult<()> {
        let Some(redirect) = redirect_uri else {
            return Ok(());
        };

        if redirect.contains('#') {
            return Err(AppError::validation(
                "redirect_uri must not contain fragment",
            ));
        }

        if !(redirect.starts_with("http://") || redirect.starts_with("https://")) {
            return Err(AppError::validation("invalid redirect_uri"));
        }

        Ok(())
    }

    fn validate_exchange_redirect_uri(stored: &Code, redirect_uri: Option<&str>) -> AppResult<()> {
        if let Some(provided) = redirect_uri
            && let Some(expected) = stored.redirect_uri.as_deref()
            && provided != expected
        {
            return Err(AppError::validation("redirect_uri mismatch"));
        }

        Ok(())
    }

    fn verify_pkce(stored: &Code, code_verifier: Option<&str>) -> AppResult<()> {
        if let Some(challenge) = stored.code_challenge.as_ref() {
            let verifier =
                code_verifier.ok_or_else(|| AppError::validation("code_verifier required"))?;

            match stored.code_challenge_method.as_deref().unwrap_or("plain") {
                "S256" | "s256" => {
                    let mut hasher = Sha256::new();
                    hasher.update(verifier.as_bytes());
                    let digest = hasher.finalize();
                    let encoded = URL_SAFE_NO_PAD.encode(&digest[..]);
                    if &encoded != challenge {
                        return Err(AppError::validation("invalid code_verifier"));
                    }
                }
                "plain" => {
                    if verifier != challenge {
                        return Err(AppError::validation("invalid code_verifier"));
                    }
                }
                other => {
                    return Err(AppError::validation(format!(
                        "unsupported code_challenge_method {other}"
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::{collections::HashSet, sync::Arc};

    use super::{
        AuthService, ExchangeAuthorizationCodeRequest, IssueAuthorizationCodeRequest,
        TokenIntrospection,
    };
    use crate::{
        application::{
            AppError, AuthTokenDto, AuthenticatedUser, TokenSubject,
            ports::{
                security::TokenManager,
                session_revocation::{Revocation, TokenVersionStore},
                time::Clock,
            },
        },
        domain::{Capability, Role, UserId, user::value_objects::Capability as UserCapability},
        infrastructure::security::{
            authorization_code_store::InMemoryStore as InMemoryAuthorizationCodeStore,
            session_store::InMemorySessionRevocationStore,
        },
    };

    #[derive(Clone)]
    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    #[derive(Clone)]
    struct StaticTokenManager {
        authenticated_user: AuthenticatedUser,
    }

    #[async_trait]
    impl TokenManager for StaticTokenManager {
        async fn issue(
            &self,
            subject: TokenSubject,
        ) -> crate::application::AppResult<AuthTokenDto> {
            let now = self.authenticated_user.issued_at;
            let expires_at = self.authenticated_user.expires_at;
            Ok(AuthTokenDto {
                token: format!("issued-{}", i64::from(subject.user_id)),
                issued_at: now,
                expires_at,
                expires_in: expires_at.signed_duration_since(now).num_seconds(),
                session_id: subject.session_id,
                refresh_token: None,
            })
        }

        async fn authenticate(
            &self,
            token: &str,
        ) -> crate::application::AppResult<AuthenticatedUser> {
            if token == "valid-token" {
                Ok(self.authenticated_user.clone())
            } else {
                Err(AppError::unauthorized("invalid token"))
            }
        }

        async fn public_jwk(&self) -> crate::application::AppResult<serde_json::Value> {
            Ok(serde_json::json!({ "keys": [] }))
        }
    }

    fn authenticated_user() -> AuthenticatedUser {
        let issued_at = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .expect("valid RFC3339")
            .with_timezone(&Utc);
        let expires_at = DateTime::parse_from_rfc3339("2024-01-01T01:00:00Z")
            .expect("valid RFC3339")
            .with_timezone(&Utc);
        let capabilities: HashSet<Capability> =
            HashSet::from([UserCapability::new("users", "read")]);

        AuthenticatedUser {
            id: UserId::new(42).expect("user id"),
            username: "tester".into(),
            role: Role::Admin,
            capabilities,
            issued_at,
            expires_at,
            session_id: Some("sid-42".into()),
            token_version: Some(1),
        }
    }

    fn build_service(
        user: AuthenticatedUser,
    ) -> (
        AuthService,
        Arc<InMemorySessionRevocationStore>,
        Arc<InMemoryAuthorizationCodeStore>,
    ) {
        let session_store = Arc::new(InMemorySessionRevocationStore::new());
        let auth_code_store = Arc::new(InMemoryAuthorizationCodeStore::new());
        let service = AuthService::new(
            Arc::new(StaticTokenManager {
                authenticated_user: user,
            }),
            session_store.clone(),
            auth_code_store.clone(),
            Arc::new(FixedClock(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .expect("valid RFC3339")
                    .with_timezone(&Utc),
            )),
        );

        (service, session_store, auth_code_store)
    }

    #[tokio::test]
    async fn authenticate_rejects_revoked_session() {
        let user = authenticated_user();
        let (service, session_store, _auth_code_store) = build_service(user);
        session_store
            .revoke("sid-42")
            .await
            .expect("revoke session");

        let err = service
            .authenticate("valid-token")
            .await
            .expect_err("revoked session should fail");

        assert!(matches!(err, AppError::Unauthorized(msg) if msg == "session revoked"));
    }

    #[tokio::test]
    async fn authenticate_rejects_revoked_token_version() {
        let user = authenticated_user();
        let (service, session_store, _auth_code_store) = build_service(user);
        session_store
            .set_min_token_version(42, 2)
            .await
            .expect("set min token version");

        let err = service
            .authenticate("valid-token")
            .await
            .expect_err("revoked token version should fail");

        assert!(matches!(err, AppError::Unauthorized(msg) if msg == "token revoked"));
    }

    #[tokio::test]
    async fn issue_authorization_code_rejects_redirect_fragments() {
        let user = authenticated_user();
        let (service, _session_store, _auth_code_store) = build_service(user.clone());

        let err = service
            .issue_authorization_code(
                &user,
                IssueAuthorizationCodeRequest {
                    client_id: None,
                    redirect_uri: Some("https://client.example/callback#fragment".into()),
                    scope: None,
                    code_challenge: None,
                    code_challenge_method: None,
                },
            )
            .await
            .expect_err("fragment redirect should be rejected");

        assert!(
            matches!(err, AppError::Validation(msg) if msg == "redirect_uri must not contain fragment")
        );
    }

    #[tokio::test]
    async fn exchange_authorization_code_validates_redirect_uri_and_pkce() {
        let user = authenticated_user();
        let (service, _session_store, _auth_code_store) = build_service(user.clone());
        let issued = service
            .issue_authorization_code(
                &user,
                IssueAuthorizationCodeRequest {
                    client_id: Some("client-id".into()),
                    redirect_uri: Some("https://client.example/callback".into()),
                    scope: Some("openid".into()),
                    code_challenge: Some("verifier".into()),
                    code_challenge_method: Some("plain".into()),
                },
            )
            .await
            .expect("issue auth code");

        let redirect_err = service
            .exchange_authorization_code(ExchangeAuthorizationCodeRequest {
                code: issued.code.clone(),
                redirect_uri: Some("https://other.example/callback".into()),
                code_verifier: Some("verifier".into()),
            })
            .await
            .expect_err("redirect mismatch should fail");
        assert!(
            matches!(redirect_err, AppError::Validation(msg) if msg == "redirect_uri mismatch")
        );

        let issued = service
            .issue_authorization_code(
                &user,
                IssueAuthorizationCodeRequest {
                    client_id: Some("client-id".into()),
                    redirect_uri: Some("https://client.example/callback".into()),
                    scope: Some("openid".into()),
                    code_challenge: Some("verifier".into()),
                    code_challenge_method: Some("plain".into()),
                },
            )
            .await
            .expect("issue auth code");

        let pkce_err = service
            .exchange_authorization_code(ExchangeAuthorizationCodeRequest {
                code: issued.code,
                redirect_uri: Some("https://client.example/callback".into()),
                code_verifier: Some("wrong".into()),
            })
            .await
            .expect_err("invalid pkce should fail");
        assert!(matches!(pkce_err, AppError::Validation(msg) if msg == "invalid code_verifier"));
    }

    #[tokio::test]
    async fn introspect_invalid_token_is_inactive() {
        let user = authenticated_user();
        let (service, _session_store, _auth_code_store) = build_service(user);

        let introspection = service
            .introspect_token("invalid-token")
            .await
            .expect("introspection should not error");

        assert_eq!(introspection, TokenIntrospection::inactive());
    }
}
