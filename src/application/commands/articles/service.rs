// src/application/commands/articles/service.rs
use std::sync::Arc;

use crate::{
    application::ports::time::Clock,
    domain::article::{
        ArticleReadRepository, ArticleRevisionRepository, ArticleWriteRepository,
        services::ArticleSlugService,
    },
};

pub struct ArticleCommandService {
    pub(super) write_repo: Arc<dyn ArticleWriteRepository>,
    pub(super) read_repo: Arc<dyn ArticleReadRepository>,
    pub(super) revision_repo: Arc<dyn ArticleRevisionRepository>,
    pub(super) slug_service: Arc<ArticleSlugService>,
    pub(super) clock: Arc<dyn Clock>,
}

impl ArticleCommandService {
    pub fn new(
        write_repo: Arc<dyn ArticleWriteRepository>,
        read_repo: Arc<dyn ArticleReadRepository>,
        revision_repo: Arc<dyn ArticleRevisionRepository>,
        slug_service: Arc<ArticleSlugService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            write_repo,
            read_repo,
            revision_repo,
            slug_service,
            clock,
        }
    }
}
