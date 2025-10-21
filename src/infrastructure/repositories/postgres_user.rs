// src/infrastructure/repositories/postgres_user.rs
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::{NewUser, PasswordHash, Role, User, UserId, UserRepository, Username};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: Arc<PgPool>,
}

impl PostgresUserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    fn map_error(err: sqlx::Error) -> DomainError {
        DomainError::Persistence(err.to_string())
    }
}

#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    role: String,
    is_active: bool,
    created_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = DomainError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: UserId::new(row.id)?,
            username: Username::new(row.username)?,
            password_hash: PasswordHash::new(row.password_hash)?,
            role: Role::from_str(&row.role)?,
            is_active: row.is_active,
            created_at: row.created_at,
        })
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn count(&self) -> DomainResult<u64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users")
            .fetch_one(&*self.pool)
            .await
            .map(|count| count as u64)
            .map_err(Self::map_error)
    }

    async fn insert(&self, new_user: NewUser) -> DomainResult<User> {
        let NewUser {
            username,
            password_hash,
            role,
            is_active,
            created_at,
        } = new_user;

        let row = sqlx::query_as::<_, UserRow>(
            "INSERT INTO users (username, password_hash, role, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, username, password_hash, role, is_active, created_at",
        )
        .bind(username.as_str())
        .bind(password_hash.as_str())
        .bind(role.as_str())
        .bind(is_active)
        .bind(created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(Self::map_error)?;

        User::try_from(row)
    }

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, is_active, created_at
             FROM users WHERE username = $1",
        )
        .bind(username.as_str())
        .fetch_optional(&*self.pool)
        .await
        .map_err(Self::map_error)?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, is_active, created_at
             FROM users WHERE id = $1",
        )
        .bind(i64::from(id))
        .fetch_optional(&*self.pool)
        .await
        .map_err(Self::map_error)?;

        row.map(User::try_from).transpose()
    }
}
