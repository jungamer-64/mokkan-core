use super::ArticleQueryService;
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser, CursorPage},
        error::{ApplicationError, ApplicationResult},
    },
    domain::{article::ArticleListCursor, errors::DomainError},
};

pub struct ListArticlesQuery {
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}

impl ArticleQueryService {
    pub async fn list_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: ListArticlesQuery,
    ) -> ApplicationResult<CursorPage<ArticleDto>> {
        let (include_drafts, limit) =
            self.normalize_listing(actor, query.include_drafts, query.limit)?;
        let cursor = self.decode_cursor(query.cursor.as_deref())?;

        let (records, next_cursor) = self
            .read_repo
            .list_page(include_drafts, limit, cursor, None)
            .await?;

        let items = records.into_iter().map(Into::into).collect();
        Ok(CursorPage::new(
            items,
            next_cursor.map(|cursor| cursor.encode()),
        ))
    }

    pub(super) fn normalize_listing(
        &self,
        actor: Option<&AuthenticatedUser>,
        include_drafts: bool,
        limit: u32,
    ) -> ApplicationResult<(bool, u32)> {
        let include_drafts = if include_drafts {
            let actor = actor.ok_or_else(|| {
                ApplicationError::forbidden("authentication required for draft access")
            })?;
            if !actor.has_capability("articles", "view:drafts") {
                return Err(ApplicationError::forbidden(
                    "missing capability articles:view:drafts",
                ));
            }
            true
        } else {
            false
        };

        const DEFAULT_LIMIT: u32 = 20;
        const MAX_LIMIT: u32 = 100;

        let limit = if limit == 0 {
            DEFAULT_LIMIT
        } else {
            limit.min(MAX_LIMIT)
        };

        Ok((include_drafts, limit))
    }

    pub(super) fn decode_cursor(
        &self,
        token: Option<&str>,
    ) -> ApplicationResult<Option<ArticleListCursor>> {
        match token {
            Some(value) => match ArticleListCursor::decode(value) {
                Ok(cursor) => Ok(Some(cursor)),
                Err(DomainError::Validation(msg)) => Err(ApplicationError::validation(msg)),
                Err(other) => Err(ApplicationError::from(other)),
            },
            None => Ok(None),
        }
    }
}
