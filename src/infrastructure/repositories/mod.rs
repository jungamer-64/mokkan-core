// src/infrastructure/repositories/mod.rs
mod error;
mod postgres_article;
mod postgres_article_revision;
mod postgres_user;
mod postgres_audit_log;

pub(crate) use error::map_sqlx;
pub use postgres_article::{PostgresArticleReadRepository, PostgresArticleWriteRepository};
pub use postgres_article_revision::PostgresArticleRevisionRepository;
pub use postgres_user::PostgresUserRepository;
pub use postgres_audit_log::PostgresAuditLogRepository;
