use super::{common, service::AuditQueryService};
use crate::{
    application::{
        AuditLogDto, AuthenticatedUser, CursorPage,
        error::{AppError, AppResult},
    },
    domain::audit::cursor::Cursor,
};

pub struct ListAuditLogsQuery {
    pub limit: u32,
    pub cursor: Option<String>,
}

pub struct ListAuditLogsByUserQuery {
    pub user_id: i64,
    pub limit: u32,
    pub cursor: Option<String>,
}

pub struct ListAuditLogsByResourceQuery {
    pub resource_type: String,
    pub resource_id: i64,
    pub limit: u32,
    pub cursor: Option<String>,
}

impl AuditQueryService {
    /// List audit logs for all resources.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks audit access, the cursor is
    /// invalid, or the repository lookup fails.
    pub async fn list_audit_logs(
        &self,
        actor: &AuthenticatedUser,
        query: ListAuditLogsQuery,
    ) -> AppResult<CursorPage<AuditLogDto>> {
        common::ensure_audit_capability(actor)?;
        let limit = common::normalize_limit(query.limit);
        let typed_cursor = Self::decode_cursor(query.cursor.as_deref())?;

        let (items, next_cursor) = self
            .repo
            .list(limit, typed_cursor)
            .await
            .map_err(AppError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }

    /// List audit logs associated with a user.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks audit access, the cursor is
    /// invalid, or the repository lookup fails.
    pub async fn list_by_user(
        &self,
        actor: &AuthenticatedUser,
        query: ListAuditLogsByUserQuery,
    ) -> AppResult<CursorPage<AuditLogDto>> {
        common::ensure_audit_capability(actor)?;
        let limit = common::normalize_limit(query.limit);
        let typed_cursor = Self::decode_cursor(query.cursor.as_deref())?;
        let (items, next_cursor) = self
            .repo
            .find_by_user(query.user_id, limit, typed_cursor)
            .await
            .map_err(AppError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }

    /// List audit logs for a specific resource.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks audit access, the cursor is
    /// invalid, or the repository lookup fails.
    pub async fn list_by_resource(
        &self,
        actor: &AuthenticatedUser,
        query: ListAuditLogsByResourceQuery,
    ) -> AppResult<CursorPage<AuditLogDto>> {
        common::ensure_audit_capability(actor)?;
        let limit = common::normalize_limit(query.limit);
        let typed_cursor = Self::decode_cursor(query.cursor.as_deref())?;
        let (items, next_cursor) = self
            .repo
            .find_by_resource(&query.resource_type, query.resource_id, limit, typed_cursor)
            .await
            .map_err(AppError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }

    fn decode_cursor(cursor: Option<&str>) -> AppResult<Option<Cursor>> {
        cursor.map_or_else(
            || Ok(None),
            |token| Ok(Some(Cursor::decode(token).map_err(AppError::from)?)),
        )
    }
}
