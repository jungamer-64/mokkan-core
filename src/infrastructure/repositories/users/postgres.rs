// src/infrastructure/repositories/users/postgres.rs
use super::super::map_sqlx;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::{
    NewUser, PasswordHash, Role, User, UserId, UserListCursor, UserRepository, UserUpdate, Username,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn build_update_query(
        &self,
        id: UserId,
        is_active: Option<bool>,
        role: Option<Role>,
        password_hash: Option<PasswordHash>,
    ) -> QueryBuilder<Postgres> {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE users SET ");
        let mut first = true;

        if let Some(is_active) = is_active {
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("is_active = ");
            builder.push_bind(is_active);
        }

        if let Some(role) = role {
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("role = ");
            builder.push_bind(role);
        }

        if let Some(password_hash) = password_hash {
            if !first {
                builder.push(", ");
            }
            first = false;
            builder.push("password_hash = ");
            let value: String = password_hash.into();
            builder.push_bind(value);
        }

        builder.push(" WHERE id = ");
        builder.push_bind(i64::from(id));
        builder.push(" RETURNING id, username, password_hash, role, is_active, created_at");

        builder
    }

    fn normalize_search(search: Option<&str>) -> Option<String> {
        search.and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("%{}%", trimmed))
            }
        })
    }
}

#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    role: Role,
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
            role: row.role,
            is_active: row.is_active,
            created_at: row.created_at,
        })
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn count(&self) -> DomainResult<u64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users")
            .fetch_one(&self.pool)
            .await
            .map(|count| count as u64)
            .map_err(map_sqlx)
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
        .bind(role)
        .bind(is_active)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;

        User::try_from(row)
    }

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, is_active, created_at
             FROM users WHERE username = $1",
        )
        .bind(username.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        row.map(User::try_from).transpose()
    }

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, username, password_hash, role, is_active, created_at
             FROM users WHERE id = $1",
        )
        .bind(i64::from(id))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        row.map(User::try_from).transpose()
    }

    async fn update(&self, update: UserUpdate) -> DomainResult<User> {
        let UserUpdate {
            id,
            is_active,
            role,
            password_hash,
        } = update;

        if is_active.is_none() && role.is_none() && password_hash.is_none() {
            return Err(DomainError::Validation(
                "no fields provided for update".into(),
            ));
        }

    let mut builder = self.build_update_query(id, is_active, role, password_hash);

        let row = builder
            .build_query_as::<UserRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
            .ok_or_else(|| DomainError::NotFound("user not found".into()))?;

        User::try_from(row)
    }

    async fn list_page(
        &self,
        limit: u32,
        cursor: Option<UserListCursor>,
        search: Option<&str>,
    ) -> DomainResult<(Vec<User>, Option<UserListCursor>)> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = (limit as i64) + 1;

        let search = Self::normalize_search(search);

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, username, password_hash, role, is_active, created_at FROM users",
        );

        let mut has_where = false;
        if let Some(pattern) = search.as_deref() {
            builder.push(" WHERE username ILIKE ");
            builder.push_bind(pattern);
            has_where = true;
        }

        if let Some(cursor) = cursor.as_ref() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("(created_at, id) < (");
            builder.push_bind(cursor.created_at);
            builder.push(", ");
            builder.push_bind(i64::from(cursor.user_id));
            builder.push(")");
        }

        builder.push(" ORDER BY created_at DESC, id DESC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build_query_as::<UserRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;

        let mut users = rows
            .into_iter()
            .map(User::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let next_cursor = if users.len() > limit as usize {
            let _ = users.pop();
            users
                .last()
                .map(|user| UserListCursor::new(user.created_at, user.id))
        } else {
            None
        };

        Ok((users, next_cursor))
    }
}
