// src/application/services/mod.rs
use std::sync::Arc;

use crate::{
    application::{
        commands::{articles::ArticleCommandService, users::UserCommandService},
        ports::{
            security::{PasswordHasher, TokenManager},
            time::Clock,
            util::SlugGenerator,
            session_revocation::SessionRevocationStore,
        },
        queries::{articles::ArticleQueryService, users::UserQueryService},
    },
    domain::{
        article::{
            ArticleReadRepository, ArticleRevisionRepository, ArticleWriteRepository,
            services::ArticleSlugService,
        },
        user::UserRepository,
    },
};

use crate::application::error::ApplicationError;
use crate::application::dto::AuthTokenDto;
use crate::application::ports::authorization_code::AuthorizationCode;
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

pub struct ApplicationServices {
    pub user_commands: Arc<UserCommandService>,
    pub article_commands: Arc<ArticleCommandService>,
    pub article_queries: Arc<ArticleQueryService>,
    pub user_queries: Arc<UserQueryService>,
    token_manager: Arc<dyn TokenManager>,
    session_revocation_store: Arc<dyn SessionRevocationStore>,
    authorization_code_store: Arc<dyn crate::application::ports::authorization_code::AuthorizationCodeStore>,
    audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
}

/// A small bundling of repository dependencies used to keep the
/// ApplicationServices::new parameter list manageable for static analysis
/// tools. Callers should construct this from their concrete repo instances
/// and pass it to `ApplicationServices::new`.
pub struct ApplicationDependencies {
    pub user_repo: Arc<dyn UserRepository>,
    pub article_write_repo: Arc<dyn ArticleWriteRepository>,
    pub article_read_repo: Arc<dyn ArticleReadRepository>,
    pub article_revision_repo: Arc<dyn ArticleRevisionRepository>,
    pub audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
}

impl ApplicationServices {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        deps: ApplicationDependencies,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        session_revocation_store: Arc<dyn SessionRevocationStore>,
        authorization_code_store: Arc<dyn crate::application::ports::authorization_code::AuthorizationCodeStore>,
        clock: Arc<dyn Clock>,
        slugger: Arc<dyn SlugGenerator>,
    ) -> Self {
        let user_commands = Arc::new(UserCommandService::new(
            Arc::clone(&deps.user_repo),
            Arc::clone(&password_hasher),
            Arc::clone(&token_manager),
            Arc::clone(&session_revocation_store),
            Arc::clone(&clock),
        ));

        let slug_service = Arc::new(ArticleSlugService::new(
            Arc::clone(&deps.article_read_repo),
            Arc::clone(&slugger),
        ));

        let article_commands = Arc::new(ArticleCommandService::new(
            Arc::clone(&deps.article_write_repo),
            Arc::clone(&deps.article_read_repo),
            Arc::clone(&deps.article_revision_repo),
            Arc::clone(&slug_service),
            Arc::clone(&clock),
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
            session_revocation_store,
            authorization_code_store,
            audit_log_repo: deps.audit_log_repo,
        }
    }

    pub fn token_manager(&self) -> Arc<dyn TokenManager> {
        Arc::clone(&self.token_manager)
    }

    pub fn session_revocation_store(&self) -> Arc<dyn SessionRevocationStore> {
        Arc::clone(&self.session_revocation_store)
    }

    pub fn authorization_code_store(&self) -> Arc<dyn crate::application::ports::authorization_code::AuthorizationCodeStore> {
        Arc::clone(&self.authorization_code_store)
    }

    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        code_verifier: Option<&str>,
        redirect_uri: Option<&str>,
    ) -> crate::application::ApplicationResult<AuthTokenDto> {
        // consume the code (single-use)
        let stored_opt = self.authorization_code_store.consume_code(code).await?;
        let stored = stored_opt.ok_or_else(|| ApplicationError::validation("invalid or expired code"))?;

        // validate redirect_uri and PKCE using helpers to keep complexity low
        self.validate_redirect_uri(&stored, redirect_uri)?;
        self.verify_pkce(&stored, code_verifier)?;

        // Issue tokens for the stored subject
        let token = self.token_manager.issue(stored.subject).await?;
        Ok(token)
    }

    fn validate_redirect_uri(
        &self,
        stored: &AuthorizationCode,
        redirect_uri: Option<&str>,
    ) -> crate::application::ApplicationResult<()> {
        if let Some(provided) = redirect_uri {
            if let Some(expected) = stored.redirect_uri.as_deref() {
                if provided != expected {
                    return Err(ApplicationError::validation("redirect_uri mismatch"));
                }
            }
        }

        Ok(())
    }

    fn verify_pkce(
        &self,
        stored: &AuthorizationCode,
        code_verifier: Option<&str>,
    ) -> crate::application::ApplicationResult<()> {
        if let Some(challenge) = stored.code_challenge.as_ref() {
            let verifier = code_verifier.ok_or_else(|| ApplicationError::validation("code_verifier required"))?;
            match stored.code_challenge_method.as_deref().unwrap_or("plain") {
                "S256" | "s256" => {
                    let mut hasher = Sha256::new();
                    hasher.update(verifier.as_bytes());
                    let digest = hasher.finalize();
                    let encoded = URL_SAFE_NO_PAD.encode(&digest[..]);
                    if &encoded != challenge {
                        return Err(ApplicationError::validation("invalid code_verifier"));
                    }
                }
                "plain" => {
                    if verifier != challenge {
                        return Err(ApplicationError::validation("invalid code_verifier"));
                    }
                }
                other => return Err(ApplicationError::validation(format!("unsupported code_challenge_method {other}"))),
            }
        }

        Ok(())
    }

    pub fn audit_log_repo(&self) -> Arc<dyn crate::domain::audit::repository::AuditLogRepository> {
        Arc::clone(&self.audit_log_repo)
    }

    /// Authenticate a raw bearer token, perform session/token-version revocation checks
    /// and ensure the authenticated subject has the specified capability.
    ///
    /// This consolidates common logic so presentation-layer middleware can simply
    /// delegate authorization checks to the application services instead of
    /// reimplementing the details.
    pub async fn authenticate_and_authorize(
        &self,
        token: &str,
        resource: &str,
        action: &str,
    ) -> crate::application::ApplicationResult<crate::application::dto::AuthenticatedUser> {
        let user = self.token_manager.authenticate(token).await?;

        // Use helper methods to keep cyclomatic complexity low for the
        // public method while keeping the individual checks explicit.
        self.ensure_session_not_revoked(&user).await?;
        self.ensure_token_version_not_revoked(&user).await?;

        self.ensure_has_capability(&user, resource, action).await?;
        Ok(user)
    }

    async fn ensure_session_not_revoked(
        &self,
        user: &crate::application::dto::AuthenticatedUser,
    ) -> crate::application::ApplicationResult<()> {
        use crate::application::error::ApplicationError;

        if let Some(session_id) = &user.session_id {
            if self.session_revocation_store.is_revoked(session_id).await? {
                return Err(ApplicationError::unauthorized("session revoked"));
            }
        }

        Ok(())
    }

    async fn ensure_token_version_not_revoked(
        &self,
        user: &crate::application::dto::AuthenticatedUser,
    ) -> crate::application::ApplicationResult<()> {
        use crate::application::error::ApplicationError;

        if let Some(token_ver) = user.token_version {
            if let Some(min_ver) = self
                .session_revocation_store
                .get_min_token_version(i64::from(user.id))
                .await?
            {
                if token_ver < min_ver {
                    return Err(ApplicationError::unauthorized("token revoked"));
                }
            }
        }

        Ok(())
    }

    async fn ensure_has_capability(
        &self,
        user: &crate::application::dto::AuthenticatedUser,
        resource: &str,
        action: &str,
    ) -> crate::application::ApplicationResult<()> {
        use crate::application::error::ApplicationError;

        if user.has_capability(resource, action) {
            Ok(())
        } else {
            Err(ApplicationError::forbidden(format!("missing capability {resource}:{action}")))
        }
    }
}
