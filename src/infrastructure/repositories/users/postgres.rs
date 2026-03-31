// src/infrastructure/repositories/users/postgres.rs
use super::super::map_sqlx;
use crate::async_support::{BoxFuture, boxed};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::{
    NewUser, PasswordHash, Role, User, UserId, UserListCursor, UserRepository, UserUpdate, Username,
};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Clone)]
#[must_use]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn build_update_query(
        id: UserId,
        is_active: Option<bool>,
        role: Option<Role>,
        password_hash: Option<PasswordHash>,
    ) -> QueryBuilder<'static, Postgres> {
        let mut builder: QueryBuilder<'static, Postgres> = QueryBuilder::new("UPDATE users SET ");
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
                Some(format!("%{trimmed}%"))
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
        Ok(Self {
            id: UserId::new(row.id)?,
            username: Username::new(row.username)?,
            password_hash: PasswordHash::new(row.password_hash)?,
            role: row.role,
            is_active: row.is_active,
            created_at: row.created_at,
        })
    }
}

impl UserRepository for PostgresUserRepository {
    fn count(&self) -> BoxFuture<'_, DomainResult<u64>> {
        boxed(async move {
            let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM users")
                .fetch_one(&self.pool)
                .await
                .map_err(map_sqlx)?;

            u64::try_from(count)
                .map_err(|_| DomainError::Persistence("user count out of range".into()))
        })
    }

    fn insert(&self, new_user: NewUser) -> BoxFuture<'_, DomainResult<User>> {
        boxed(async move {
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
        })
    }

    fn find_by_username<'a>(
        &'a self,
        username: &'a Username,
    ) -> BoxFuture<'a, DomainResult<Option<User>>> {
        boxed(async move {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, username, password_hash, role, is_active, created_at
                 FROM users WHERE username = $1",
            )
            .bind(username.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;

            row.map(User::try_from).transpose()
        })
    }

    fn find_by_id(&self, id: UserId) -> BoxFuture<'_, DomainResult<Option<User>>> {
        boxed(async move {
            let row = sqlx::query_as::<_, UserRow>(
                "SELECT id, username, password_hash, role, is_active, created_at
                 FROM users WHERE id = $1",
            )
            .bind(i64::from(id))
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;

            row.map(User::try_from).transpose()
        })
    }

    fn update(&self, update: UserUpdate) -> BoxFuture<'_, DomainResult<User>> {
        boxed(async move {
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

            let mut builder = Self::build_update_query(id, is_active, role, password_hash);

            let row = builder
                .build_query_as::<UserRow>()
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx)?
                .ok_or_else(|| DomainError::NotFound("user not found".into()))?;

            User::try_from(row)
        })
    }

    fn list_page<'a>(
        &'a self,
        limit: u32,
        cursor: Option<UserListCursor>,
        search: Option<&'a str>,
    ) -> BoxFuture<'a, DomainResult<(Vec<User>, Option<UserListCursor>)>> {
        boxed(async move {
            let limit = limit.clamp(1, 100);
            let fetch_limit = i64::from(limit) + 1;

            let search = Self::normalize_search(search);

            let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "SELECT id, username, password_hash, role, is_active, created_at FROM users",
            );

            let has_where = search.as_deref().is_some_and(|pattern| {
                builder.push(" WHERE username ILIKE ");
                builder.push_bind(pattern);
                true
            });

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
        })
    }
}
