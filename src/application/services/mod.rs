// src/application/services/mod.rs
use std::sync::Arc;

use crate::{
    application::{
        AuthTokenDto,
        commands::{articles::ArticleCommandService, users::UserCommandService},
        error::AppError,
        ports::authorization_code::{Code, CodeStore},
        ports::{
            refresh_token::Codec,
            security::{PasswordHasher, TokenManager},
            session_revocation::{
                Ports, Revocation, SessionMetadataStore, Store, TokenVersionStore,
            },
            time::Clock,
            util::SlugGenerator,
        },
        queries::{articles::ArticleQueryService, users::UserQueryService},
    },
    domain::{
        ArticleReadRepository, ArticleRevisionRepository, ArticleWriteRepository, UserRepository,
        article::services::ArticleSlugService,
    },
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

#[must_use]
pub struct Registry {
    pub user_commands: Arc<UserCommandService>,
    pub article_commands: Arc<ArticleCommandService>,
    pub article_queries: Arc<ArticleQueryService>,
    pub user_queries: Arc<UserQueryService>,
    token_manager: Arc<dyn TokenManager>,
    session_stores: Ports,
    session_revocation_store: Arc<dyn Store>,
    authorization_code_store: Arc<dyn CodeStore>,
    audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
}

/// A small bundle of repository dependencies for `Registry::new`.
///
/// This keeps the constructor parameter list manageable for static analysis
/// tools. Callers should construct this from their concrete repo instances.
pub struct Dependencies {
    pub user_repo: Arc<dyn UserRepository>,
    pub article_write_repo: Arc<dyn ArticleWriteRepository>,
    pub article_read_repo: Arc<dyn ArticleReadRepository>,
    pub article_revision_repo: Arc<dyn ArticleRevisionRepository>,
    pub audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
}

/// Runtime-facing collaborators required to build `Registry`.
pub struct RuntimeDependencies {
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub token_manager: Arc<dyn TokenManager>,
    pub refresh_token_codec: Arc<dyn Codec>,
    pub session_revocation_store: Arc<dyn Store>,
    pub authorization_code_store: Arc<dyn CodeStore>,
    pub clock: Arc<dyn Clock>,
    pub slugger: Arc<dyn SlugGenerator>,
}

impl Registry {
    pub fn new(deps: Dependencies, runtime: RuntimeDependencies) -> Self {
        let RuntimeDependencies {
            password_hasher,
            token_manager,
            refresh_token_codec,
            session_revocation_store,
            authorization_code_store,
            clock,
            slugger,
        } = runtime;
        let session_stores = Ports::from_store(Arc::clone(&session_revocation_store));
        let user_commands = Arc::new(UserCommandService::new(
            Arc::clone(&deps.user_repo),
            password_hasher,
            Arc::clone(&token_manager),
            refresh_token_codec,
            Arc::clone(&session_revocation_store),
            Arc::clone(&clock),
        ));

        let slug_service = Arc::new(ArticleSlugService::new(
            Arc::clone(&deps.article_read_repo),
            slugger,
        ));

        let article_commands = Arc::new(ArticleCommandService::new(
            Arc::clone(&deps.article_write_repo),
            Arc::clone(&deps.article_read_repo),
            Arc::clone(&deps.article_revision_repo),
            Arc::clone(&slug_service),
            clock,
        ));

        let article_queries = Arc::new(ArticleQueryService::new(
            Arc::clone(&deps.article_read_repo),
            Arc::clone(&deps.article_revision_repo),
        ));
        let user_queries = Arc::new(UserQueryService::new(Arc::clone(&deps.user_repo)));

        Self {
            user_commands,
            article_commands,
            article_queries,
            user_queries,
            token_manager,
            session_stores,
            session_revocation_store,
            authorization_code_store,
            audit_log_repo: deps.audit_log_repo,
        }
    }

    #[must_use]
    pub fn token_manager(&self) -> Arc<dyn TokenManager> {
        Arc::clone(&self.token_manager)
    }

    #[must_use]
    pub fn session_revocation_store(&self) -> Arc<dyn Store> {
        Arc::clone(&self.session_revocation_store)
    }

    #[must_use]
    pub fn session_revocation(&self) -> Arc<dyn Revocation> {
        Arc::clone(&self.session_stores.revocation)
    }

    #[must_use]
    pub fn token_version_store(&self) -> Arc<dyn TokenVersionStore> {
        Arc::clone(&self.session_stores.token_versions)
    }

    #[must_use]
    pub fn session_metadata_store(&self) -> Arc<dyn SessionMetadataStore> {
        Arc::clone(&self.session_stores.session_metadata)
    }

    #[must_use]
    pub fn authorization_code_store(&self) -> Arc<dyn CodeStore> {
        Arc::clone(&self.authorization_code_store)
    }

    /// Exchange an authorization code for tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if the code is missing, expired, already consumed,
    /// the redirect URI or PKCE verifier is invalid, or token issuance fails.
    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        code_verifier: Option<&str>,
        redirect_uri: Option<&str>,
    ) -> crate::application::AppResult<AuthTokenDto> {
        // consume the code (single-use)
        let stored_opt = self.authorization_code_store.consume_code(code).await?;
        let stored = stored_opt.ok_or_else(|| AppError::validation("invalid or expired code"))?;

        // validate redirect_uri and PKCE using helpers to keep complexity low
        Self::validate_redirect_uri(&stored, redirect_uri)?;
        Self::verify_pkce(&stored, code_verifier)?;

        // Issue tokens for the stored subject
        let token = self.token_manager.issue(stored.subject).await?;
        Ok(token)
    }

    fn validate_redirect_uri(
        stored: &Code,
        redirect_uri: Option<&str>,
    ) -> crate::application::AppResult<()> {
        if let Some(provided) = redirect_uri
            && let Some(expected) = stored.redirect_uri.as_deref()
            && provided != expected
        {
            return Err(AppError::validation("redirect_uri mismatch"));
        }

        Ok(())
    }

    fn verify_pkce(
        stored: &Code,
        code_verifier: Option<&str>,
    ) -> crate::application::AppResult<()> {
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

    #[must_use]
    pub fn audit_log_repo(&self) -> Arc<dyn crate::domain::audit::repository::AuditLogRepository> {
        Arc::clone(&self.audit_log_repo)
    }

    /// Authenticate a raw bearer token, perform session/token-version revocation checks
    /// and ensure the authenticated subject has the specified capability.
    ///
    /// This consolidates common logic so presentation-layer middleware can simply
    /// delegate authorization checks to the application services instead of
    /// reimplementing the details.
    ///
    /// # Errors
    ///
    /// Returns an error if the token is invalid, revoked, expired, or the
    /// authenticated user lacks the requested capability.
    pub async fn authenticate_and_authorize(
        &self,
        token: &str,
        resource: &str,
        action: &str,
    ) -> crate::application::AppResult<crate::application::AuthenticatedUser> {
        let user = self.token_manager.authenticate(token).await?;

        // Use helper methods to keep cyclomatic complexity low for the
        // public method while keeping the individual checks explicit.
        self.ensure_session_not_revoked(&user).await?;
        self.ensure_token_version_not_revoked(&user).await?;

        Self::ensure_has_capability(&user, resource, action)?;
        Ok(user)
    }

    async fn ensure_session_not_revoked(
        &self,
        user: &crate::application::AuthenticatedUser,
    ) -> crate::application::AppResult<()> {
        use crate::application::error::AppError;

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

    async fn ensure_token_version_not_revoked(
        &self,
        user: &crate::application::AuthenticatedUser,
    ) -> crate::application::AppResult<()> {
        use crate::application::error::AppError;

        if let Some(token_ver) = user.token_version
            && let Some(min_ver) = self
                .session_stores
                .token_versions
                .get_min_token_version(i64::from(user.id))
                .await?
            && token_ver < min_ver
        {
            return Err(AppError::unauthorized("token revoked"));
        }

        Ok(())
    }

    fn ensure_has_capability(
        user: &crate::application::AuthenticatedUser,
        resource: &str,
        action: &str,
    ) -> crate::application::AppResult<()> {
        use crate::application::error::AppError;

        if user.has_capability(resource, action) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "missing capability {resource}:{action}"
            )))
        }
    }
}
