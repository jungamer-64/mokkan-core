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

pub struct ApplicationServices {
    pub user_commands: Arc<UserCommandService>,
    pub article_commands: Arc<ArticleCommandService>,
    pub article_queries: Arc<ArticleQueryService>,
    pub user_queries: Arc<UserQueryService>,
    token_manager: Arc<dyn TokenManager>,
    session_revocation_store: Arc<dyn SessionRevocationStore>,
    audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
}

impl ApplicationServices {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        article_write_repo: Arc<dyn ArticleWriteRepository>,
        article_read_repo: Arc<dyn ArticleReadRepository>,
        article_revision_repo: Arc<dyn ArticleRevisionRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        session_revocation_store: Arc<dyn SessionRevocationStore>,
        audit_log_repo: Arc<dyn crate::domain::audit::repository::AuditLogRepository>,
        clock: Arc<dyn Clock>,
        slugger: Arc<dyn SlugGenerator>,
    ) -> Self {
        let user_commands = Arc::new(UserCommandService::new(
            Arc::clone(&user_repo),
            Arc::clone(&password_hasher),
            Arc::clone(&token_manager),
            Arc::clone(&clock),
        ));

        let slug_service = Arc::new(ArticleSlugService::new(
            Arc::clone(&article_read_repo),
            Arc::clone(&slugger),
        ));

        let article_commands = Arc::new(ArticleCommandService::new(
            Arc::clone(&article_write_repo),
            Arc::clone(&article_read_repo),
            Arc::clone(&article_revision_repo),
            Arc::clone(&slug_service),
            Arc::clone(&clock),
        ));

        let article_queries = Arc::new(ArticleQueryService::new(
            Arc::clone(&article_read_repo),
            Arc::clone(&article_revision_repo),
        ));
        let user_queries = Arc::new(UserQueryService::new(Arc::clone(&user_repo)));

        Self {
            user_commands,
            article_commands,
            article_queries,
            user_queries,
            token_manager,
            session_revocation_store,
            audit_log_repo,
        }
    }

    pub fn token_manager(&self) -> Arc<dyn TokenManager> {
        Arc::clone(&self.token_manager)
    }

    pub fn session_revocation_store(&self) -> Arc<dyn SessionRevocationStore> {
        Arc::clone(&self.session_revocation_store)
    }

    pub fn audit_log_repo(&self) -> Arc<dyn crate::domain::audit::repository::AuditLogRepository> {
        Arc::clone(&self.audit_log_repo)
    }
}
