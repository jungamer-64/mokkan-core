// src/domain/article/value_objects.rs
use crate::domain::errors::{DomainError, DomainResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArticleId(pub i64);

impl ArticleId {
    pub fn new(id: i64) -> DomainResult<Self> {
        if id <= 0 {
            Err(DomainError::Validation(
                "article id must be positive".into(),
            ))
        } else {
            Ok(Self(id))
        }
    }
}

impl From<ArticleId> for i64 {
    fn from(value: ArticleId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleTitle(String);

impl ArticleTitle {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("title cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the value object and return the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ArticleTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ArticleTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<ArticleTitle> for String {
    fn from(value: ArticleTitle) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleSlug(String);

impl ArticleSlug {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("slug cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the value object and return the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ArticleSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ArticleSlug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<ArticleSlug> for String {
    fn from(value: ArticleSlug) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleBody(String);

impl ArticleBody {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("body cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the value object and return the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ArticleBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ArticleBody {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<ArticleBody> for String {
    fn from(value: ArticleBody) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleListCursor {
    pub created_at: DateTime<Utc>,
    pub article_id: ArticleId,
}

impl ArticleListCursor {
    pub fn new(created_at: DateTime<Utc>, article_id: ArticleId) -> Self {
        Self {
            created_at,
            article_id,
        }
    }

    pub fn from_parts(created_at: DateTime<Utc>, article_id: ArticleId) -> Self {
        Self::new(created_at, article_id)
    }

    pub fn encode(&self) -> String {
        let raw = format!(
            "{}|{}",
            self.created_at.to_rfc3339(),
            i64::from(self.article_id)
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(token: &str) -> DomainResult<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        let raw = String::from_utf8(bytes)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;

        let mut parts = raw.splitn(2, '|');
        let ts_part = parts
            .next()
            .ok_or_else(|| DomainError::Validation("invalid cursor token".into()))?;
        let id_part = parts
            .next()
            .ok_or_else(|| DomainError::Validation("invalid cursor token".into()))?;

        let created_at = DateTime::parse_from_rfc3339(ts_part)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?
            .with_timezone(&Utc);
        let id_value = id_part
            .parse::<i64>()
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        let article_id = ArticleId::new(id_value)?;

        Ok(Self {
            created_at,
            article_id,
        })
    }
}
