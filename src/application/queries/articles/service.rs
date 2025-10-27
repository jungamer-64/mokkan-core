use std::sync::Arc;

use crate::domain::article::{ArticleReadRepository, ArticleRevisionRepository};

pub struct ArticleQueryService {
    pub(super) read_repo: Arc<dyn ArticleReadRepository>,
    pub(super) revision_repo: Arc<dyn ArticleRevisionRepository>,
}

impl ArticleQueryService {
    pub fn new(
        read_repo: Arc<dyn ArticleReadRepository>,
        revision_repo: Arc<dyn ArticleRevisionRepository>,
    ) -> Self {
        Self {
            read_repo,
            revision_repo,
        }
    }
}
