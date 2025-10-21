// src/application/queries/users.rs
use crate::{
    application::{
        dto::{AuthenticatedUser, CursorPage, UserDto, UserProfileDto},
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::{UserListCursor, UserRepository},
};
use std::sync::Arc;

pub struct UserQueryService {
    user_repo: Arc<dyn UserRepository>,
}

pub struct ListUsersQuery {
    pub limit: u32,
    pub cursor: Option<String>,
    pub q: Option<String>,
}

impl UserQueryService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn get_profile(
        &self,
        actor: &AuthenticatedUser,
    ) -> ApplicationResult<UserProfileDto> {
        let user = self
            .user_repo
            .find_by_id(actor.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        Ok(UserProfileDto::from_parts(user, actor))
    }

    pub async fn list_users(
        &self,
        actor: &AuthenticatedUser,
        query: ListUsersQuery,
    ) -> ApplicationResult<CursorPage<UserDto>> {
        if !actor.has_capability("users", "read") {
            return Err(ApplicationError::forbidden("missing capability users:read"));
        }

        let limit = normalize_limit(query.limit);
        let cursor = decode_cursor(query.cursor.as_deref())?;

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

fn decode_cursor(token: Option<&str>) -> ApplicationResult<Option<UserListCursor>> {
    match token {
        Some(value) => UserListCursor::decode(value)
            .map(Some)
            .map_err(ApplicationError::from),
        None => Ok(None),
    }
}
