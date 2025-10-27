use crate::application::dto::{ArticleDto, CursorPage, UserDto};
use serde::{Deserialize, Serialize};

// Simple status response used by health endpoints and docs.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusResponse {
    pub status: String,
}

// ---- response wrappers used for OpenAPI schemas ----
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl From<CursorPage<UserDto>> for UserListResponse {
    fn from(page: CursorPage<UserDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ArticleListResponse {
    pub items: Vec<ArticleDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl From<CursorPage<ArticleDto>> for ArticleListResponse {
    fn from(page: CursorPage<ArticleDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}
