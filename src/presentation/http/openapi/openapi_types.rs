//! OpenAPI response types used by the documentation generation.
//!
//! These are lightweight wrappers around application DTOs to expose stable
//! response schemas for the OpenAPI document.
use crate::application::dto::{ArticleDto, CursorPage, UserDto};
use serde::{Deserialize, Serialize};

// Simple status response used by health endpoints and docs.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
/// A minimal status response returned by health checks and exposed in the OpenAPI
/// document. The `status` field is intentionally simple and meant for humans
/// and lightweight monitoring systems.
pub struct StatusResponse {
    /// A short human-readable status string, commonly "ok" when healthy.
    pub status: String,
}

// ---- response wrappers used for OpenAPI schemas ----
#[derive(Debug, Serialize, utoipa::ToSchema)]
/// Paginated list of users for endpoints that return a cursor-based page.
pub struct UserListResponse {
    /// The list of users contained in this page.
    pub items: Vec<UserDto>,
    /// An opaque cursor string to retrieve the next page, if any.
    pub next_cursor: Option<String>,
    /// True when there are more items available after this page.
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
/// Paginated list of articles for endpoints that return a cursor-based page.
pub struct ArticleListResponse {
    /// The list of articles contained in this page.
    pub items: Vec<ArticleDto>,
    /// An opaque cursor string to retrieve the next page, if any.
    pub next_cursor: Option<String>,
    /// True when there are more items available after this page.
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
