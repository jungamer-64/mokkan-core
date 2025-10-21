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
