use super::ArticleQueryService;
use crate::{
    application::{
        dto::ArticleDto,
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::ArticleId,
};

pub struct GetArticleByIdQuery {
    pub id: i64,
}

impl ArticleQueryService {
    pub async fn get_article_by_id(
        &self,
        query: GetArticleByIdQuery,
    ) -> ApplicationResult<ArticleDto> {
        let id = ArticleId::new(query.id)?;
        let article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;
        Ok(article.into())
    }
}
