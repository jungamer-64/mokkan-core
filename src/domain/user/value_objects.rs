// src/domain/user/value_objects.rs
use crate::domain::errors::{DomainError, DomainResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::{collections::HashSet, fmt, str::FromStr};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub i64);

impl UserId {
    pub fn new(id: i64) -> DomainResult<Self> {
        if id <= 0 {
            Err(DomainError::Validation("user id must be positive".into()))
        } else {
            Ok(Self(id))
        }
    }
}

impl From<UserId> for i64 {
    fn from(value: UserId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub resource: String,
    pub action: String,
}

impl Capability {
    pub fn new(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
        }
    }

    pub fn matches(&self, resource: &str, action: &str) -> bool {
        self.resource == resource && self.action == action
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Type, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Author,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Author => "author",
        }
    }

    pub fn default_capabilities(&self) -> HashSet<Capability> {
        use Capability as Cap;
        match self {
            Role::Admin => HashSet::from([
                Cap::new("articles", "create"),
                Cap::new("articles", "update:any"),
                Cap::new("articles", "delete:any"),
                Cap::new("articles", "publish"),
                Cap::new("articles", "view:drafts"),
                Cap::new("users", "create"),
                Cap::new("users", "read"),
                Cap::new("users", "update"),
            ]),
            Role::Author => HashSet::from([
                Cap::new("articles", "create"),
                Cap::new("articles", "update:own"),
                Cap::new("articles", "delete:own"),
                Cap::new("articles", "publish"),
                Cap::new("articles", "view:drafts"),
            ]),
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Role::Author
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "author" => Ok(Role::Author),
            other => Err(DomainError::Validation(format!("unknown role '{other}'"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Username(String);

impl Username {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("username cannot be empty".into()));
        }
        if value.len() < 3 {
            return Err(DomainError::Validation(
                "username must be at least 3 characters long".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Username {
    /// Consume the Username and return the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(DomainError::Validation(
                "password hash cannot be empty".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<PasswordHash> for String {
    fn from(value: PasswordHash) -> Self {
        value.0
    }
}

impl PasswordHash {
    /// Consume the PasswordHash and return the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for PasswordHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserListCursor {
    pub created_at: DateTime<Utc>,
    pub user_id: UserId,
}

impl UserListCursor {
    pub fn new(created_at: DateTime<Utc>, user_id: UserId) -> Self {
        Self {
            created_at,
            user_id,
        }
    }

    pub fn encode(&self) -> String {
        let raw = format!(
            "{}|{}",
            self.created_at.to_rfc3339(),
            i64::from(self.user_id)
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
        let id = id_part
            .parse::<i64>()
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        let user_id = UserId::new(id)?;

        Ok(Self {
            created_at,
            user_id,
        })
    }
}
