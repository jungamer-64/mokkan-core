use crate::domain::errors::{DomainError, DomainResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, str::FromStr};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
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

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    pub password_hash: PasswordHash,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn activate(mut self) -> Self {
        self.is_active = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: Username,
    pub password_hash: PasswordHash,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl NewUser {
    pub fn new(
        username: Username,
        password_hash: PasswordHash,
        role: Role,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            username,
            password_hash,
            role,
            is_active: true,
            created_at,
        }
    }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn count(&self) -> DomainResult<u64>;

    async fn insert(&self, new_user: NewUser) -> DomainResult<User>;

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>>;

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>>;
}
