use super::ArticleQueryService;
use crate::{
    application::{
        ArticleDto, AuthenticatedUser, CursorPage,
        error::{AppError, AppResult},
    },
    domain::{ArticleListCursor, errors::DomainError},
};

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;

pub struct ListArticlesQuery {
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}

impl ArticleQueryService {
    /// List articles with optional draft visibility.
    ///
    /// # Errors
    ///
    /// Returns an error if draft access is not allowed, the cursor is invalid,
    /// or the repository lookup fails.
    pub async fn list_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: ListArticlesQuery,
    ) -> AppResult<CursorPage<ArticleDto>> {
        let (include_drafts, limit) =
            Self::normalize_listing(actor, query.include_drafts, query.limit)?;
        let cursor = Self::decode_cursor(query.cursor.as_deref())?;

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
        actor: Option<&AuthenticatedUser>,
        include_drafts: bool,
        limit: u32,
    ) -> AppResult<(bool, u32)> {
        let include_drafts = if include_drafts {
            let actor = actor
                .ok_or_else(|| AppError::forbidden("authentication required for draft access"))?;
            if !actor.has_capability("articles", "view:drafts") {
                return Err(AppError::forbidden(
                    "missing capability articles:view:drafts",
                ));
            }
            true
        } else {
            false
        };

        let limit = if limit == 0 {
            DEFAULT_LIMIT
        } else {
            limit.min(MAX_LIMIT)
        };

        Ok((include_drafts, limit))
    }

    pub(super) fn decode_cursor(token: Option<&str>) -> AppResult<Option<ArticleListCursor>> {
        token.map_or_else(
            || Ok(None),
            |value| match ArticleListCursor::decode(value) {
                Ok(cursor) => Ok(Some(cursor)),
                Err(DomainError::Validation(msg)) => Err(AppError::validation(msg)),
                Err(other) => Err(AppError::from(other)),
            },
        )
    }
}
