use super::UserQueryService;
use crate::{
    application::{
        AuthenticatedUser, CursorPage, UserDto,
        error::{AppError, AppResult},
    },
    domain::UserListCursor,
};

pub struct ListUsersQuery {
    pub limit: u32,
    pub cursor: Option<String>,
    pub q: Option<String>,
}

impl UserQueryService {
    /// List users visible to an authenticated admin-like caller.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks `users:read`, the cursor is
    /// invalid, or the repository lookup fails.
    pub async fn list_users(
        &self,
        actor: &AuthenticatedUser,
        query: ListUsersQuery,
    ) -> AppResult<CursorPage<UserDto>> {
        if !actor.has_capability("users", "read") {
            return Err(AppError::forbidden("missing capability users:read"));
        }

        let limit = Self::normalize_limit(query.limit);
        let cursor = Self::decode_cursor(query.cursor.as_deref())?;

        let (users, next_cursor) = self
            .user_repo
            .list_page(limit, cursor, query.q.as_deref())
            .await?;

        let items = users.into_iter().map(Into::into).collect();
        Ok(CursorPage::new(
            items,
            next_cursor.map(|cursor| cursor.encode()),
        ))
    }

    fn normalize_limit(limit: u32) -> u32 {
        const DEFAULT_LIMIT: u32 = 20;
        const MAX_LIMIT: u32 = 100;

        if limit == 0 {
            DEFAULT_LIMIT
        } else {
            limit.min(MAX_LIMIT)
        }
    }

    fn decode_cursor(token: Option<&str>) -> AppResult<Option<UserListCursor>> {
        token.map_or_else(
            || Ok(None),
            |value| {
                UserListCursor::decode(value)
                    .map(Some)
                    .map_err(AppError::from)
            },
        )
    }
}
