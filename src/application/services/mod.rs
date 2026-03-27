use std::sync::Arc;

use crate::{
    application::{
        AuthTokenDto, AuthenticatedUser,
        commands::{articles::ArticleCommandService, users::UserCommandService},
        ports::{
            authorization_code::CodeStore,
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

mod auth;
mod session;

pub use auth::{
    AuthService, ExchangeAuthorizationCodeRequest, IssueAuthorizationCodeRequest,
    IssueAuthorizationCodeResult, TokenIntrospection,
};
pub use session::{ListSessionsRequest, RevokeSessionRequest, SessionService};

#[must_use]
pub struct Registry {
    pub user_commands: Arc<UserCommandService>,
    pub article_commands: Arc<ArticleCommandService>,
    pub article_queries: Arc<ArticleQueryService>,
    pub user_queries: Arc<UserQueryService>,
    pub auth: Arc<AuthService>,
    pub sessions: Arc<SessionService>,
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
            Arc::clone(&clock),
        ));

        let article_queries = Arc::new(ArticleQueryService::new(
            Arc::clone(&deps.article_read_repo),
            Arc::clone(&deps.article_revision_repo),
        ));
        let user_queries = Arc::new(UserQueryService::new(Arc::clone(&deps.user_repo)));
        let auth = Arc::new(AuthService::new(
            Arc::clone(&token_manager),
            Arc::clone(&session_revocation_store),
            Arc::clone(&authorization_code_store),
            Arc::clone(&clock),
        ));
        let sessions = Arc::new(SessionService::new(
            Arc::clone(&session_revocation_store),
            clock,
        ));

        Self {
            user_commands,
            article_commands,
            article_queries,
            user_queries,
            auth,
            sessions,
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

    /// Backwards-compatible wrapper that delegates authorization code exchange
    /// to the dedicated auth service.
    ///
    /// # Errors
    ///
    /// Returns an error if the code cannot be exchanged.
    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        code_verifier: Option<&str>,
        redirect_uri: Option<&str>,
    ) -> crate::application::AppResult<AuthTokenDto> {
        self.auth
            .exchange_authorization_code(ExchangeAuthorizationCodeRequest {
                code: code.to_string(),
                code_verifier: code_verifier.map(std::string::ToString::to_string),
                redirect_uri: redirect_uri.map(std::string::ToString::to_string),
            })
            .await
    }

    #[must_use]
    pub fn audit_log_repo(&self) -> Arc<dyn crate::domain::audit::repository::AuditLogRepository> {
        Arc::clone(&self.audit_log_repo)
    }

    /// Backwards-compatible wrapper that delegates token authentication and
    /// capability checks to the dedicated auth service.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication or authorization fails.
    pub async fn authenticate_and_authorize(
        &self,
        token: &str,
        resource: &str,
        action: &str,
    ) -> crate::application::AppResult<AuthenticatedUser> {
        self.auth
            .authenticate_and_authorize(token, resource, action)
            .await
    }
}
