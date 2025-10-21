// src/domain/user/entity.rs
use crate::domain::errors::DomainResult;
use crate::domain::user::value_objects::{PasswordHash, Role, UserId, Username};
use chrono::{DateTime, Utc};

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
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    pub fn set_role(&mut self, role: Role) {
        self.role = role;
    }

    pub fn set_password(&mut self, password_hash: PasswordHash) {
        self.password_hash = password_hash;
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
    ) -> DomainResult<Self> {
        Ok(Self {
            username,
            password_hash,
            role,
            is_active: true,
            created_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UserUpdate {
    pub id: UserId,
    pub is_active: Option<bool>,
    pub role: Option<Role>,
    pub password_hash: Option<PasswordHash>,
}

impl UserUpdate {
    pub fn new(id: UserId) -> Self {
        Self {
            id,
            is_active: None,
            role: None,
            password_hash: None,
        }
    }

    pub fn with_is_active(mut self, is_active: bool) -> Self {
        self.is_active = Some(is_active);
        self
    }

    pub fn with_role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_password_hash(mut self, password_hash: PasswordHash) -> Self {
        self.password_hash = Some(password_hash);
        self
    }
}
