// src/infrastructure/repositories/mod.rs
pub mod articles;
pub mod audit;
mod error;
pub mod users;

pub use articles::{
    PostgresArticleReadRepository, PostgresArticleRevisionRepository,
    PostgresArticleWriteRepository,
};
pub use audit::PostgresAuditLogRepository;
pub(crate) use error::map_sqlx;
pub use users::PostgresUserRepository;
