use super::ArticleQueryService;
use crate::{
    application::{
        ArticleDto,
        error::{AppError, AppResult},
    },
    domain::ArticleId,
};

pub struct GetArticleByIdQuery {
    pub id: i64,
}

impl ArticleQueryService {
    /// Load an article by its numeric id.
    ///
    /// # Errors
    ///
    /// Returns an error if the id is invalid, the article does not exist, or
    /// the repository lookup fails.
    pub async fn get_article_by_id(&self, query: GetArticleByIdQuery) -> AppResult<ArticleDto> {
        let id = ArticleId::new(query.id)?;
        let article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("article not found"))?;
        Ok(article.into())
    }
}
