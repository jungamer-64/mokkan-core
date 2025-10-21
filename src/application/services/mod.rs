use std::sync::Arc;

use crate::{
    application::{
        commands::{articles::ArticleCommandService, users::UserCommandService},
        ports::{
            security::{PasswordHasher, TokenManager},
            time::Clock,
            util::SlugGenerator,
        },
        queries::{articles::ArticleQueryService, users::UserQueryService},
    },
    domain::{
        article::{ArticleReadRepository, ArticleWriteRepository},
        user::UserRepository,
    },
};

pub struct ApplicationServices {
    pub user_commands: Arc<UserCommandService>,
    pub article_commands: Arc<ArticleCommandService>,
    pub article_queries: Arc<ArticleQueryService>,
    pub user_queries: Arc<UserQueryService>,
    token_manager: Arc<dyn TokenManager>,
}

impl ApplicationServices {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        article_write_repo: Arc<dyn ArticleWriteRepository>,
        article_read_repo: Arc<dyn ArticleReadRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        clock: Arc<dyn Clock>,
        slugger: Arc<dyn SlugGenerator>,
    ) -> Self {
        let user_commands = Arc::new(UserCommandService::new(
            Arc::clone(&user_repo),
            Arc::clone(&password_hasher),
            Arc::clone(&token_manager),
            Arc::clone(&clock),
        ));

        let article_commands = Arc::new(ArticleCommandService::new(
            Arc::clone(&article_write_repo),
            Arc::clone(&article_read_repo),
            Arc::clone(&slugger),
            Arc::clone(&clock),
        ));

        let article_queries = Arc::new(ArticleQueryService::new(Arc::clone(&article_read_repo)));
        let user_queries = Arc::new(UserQueryService::new(Arc::clone(&user_repo)));

        Self {
            user_commands,
            article_commands,
            article_queries,
            user_queries,
            token_manager,
        }
    }

    pub fn token_manager(&self) -> Arc<dyn TokenManager> {
        Arc::clone(&self.token_manager)
    }
}
