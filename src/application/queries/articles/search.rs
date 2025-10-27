use super::{ArticleQueryService, list::ListArticlesQuery};
use crate::application::{
    dto::{ArticleDto, AuthenticatedUser, CursorPage},
    error::ApplicationResult,
};

pub struct SearchArticlesQuery {
    pub query: String,
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}

impl ArticleQueryService {
    pub async fn search_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: SearchArticlesQuery,
    ) -> ApplicationResult<CursorPage<ArticleDto>> {
        let trimmed = query.query.trim();
        if trimmed.is_empty() {
            return self
                .list_articles(
                    actor,
                    ListArticlesQuery {
                        include_drafts: query.include_drafts,
                        limit: query.limit,
                        cursor: query.cursor,
                    },
                )
                .await;
        }

        let (include_drafts, limit) =
            self.normalize_listing(actor, query.include_drafts, query.limit)?;
        let cursor = self.decode_cursor(query.cursor.as_deref())?;

        let (records, next_cursor) = self
            .read_repo
            .list_page(include_drafts, limit, cursor, Some(trimmed))
            .await?;

        let items = records.into_iter().map(Into::into).collect();
        Ok(CursorPage::new(
            items,
            next_cursor.map(|cursor| cursor.encode()),
        ))
    }
}
