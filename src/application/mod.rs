// src/application/mod.rs
pub mod commands;
pub mod dto;
pub mod error;
pub mod ports;
pub mod queries;
pub(crate) mod random_id;
pub mod services;

pub use dto::articles::{ArticleDto, ArticleRevisionDto};
pub use dto::audit::LogDto as AuditLogDto;
pub use dto::auth::{
    Subject as TokenSubject, TokenDto as AuthTokenDto, UserIdentity as AuthenticatedUser,
};
pub use dto::pagination::CursorPage;
pub use dto::sessions::SessionInfoDto;
pub use dto::users::{CapabilityView, UserDto, UserProfileDto};
pub use error::{AppError, AppResult};
