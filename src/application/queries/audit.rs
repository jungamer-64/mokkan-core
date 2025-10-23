// src/application/queries/audit.rs
use crate::application::dto::{AuditLogDto, CursorPage};
use crate::application::error::{ApplicationError, ApplicationResult};
use crate::domain::audit::repository::AuditLogRepository;
use crate::application::dto::AuthenticatedUser;
use std::sync::Arc;
use crate::domain::audit::cursor::AuditLogCursor;

pub struct AuditQueryService {
    repo: Arc<dyn AuditLogRepository>,
}

pub struct ListAuditQuery {
    pub limit: u32,
    pub cursor: Option<String>,
}

impl AuditQueryService {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_audit_logs(&self, actor: &AuthenticatedUser, query: ListAuditQuery) -> ApplicationResult<CursorPage<AuditLogDto>> {
        if !actor.has_capability("audit", "read") {
            return Err(ApplicationError::forbidden("missing capability audit:read"));
        }

        let limit = normalize_limit(query.limit);
        let typed_cursor = match query.cursor {
            Some(ref t) => Some(AuditLogCursor::decode(t).map_err(ApplicationError::from)?),
            None => None,
        };
        let (items, next_cursor) = self.repo.list(limit, typed_cursor).await.map_err(ApplicationError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }

    pub async fn list_by_user(&self, actor: &AuthenticatedUser, user_id: i64, query: ListAuditQuery) -> ApplicationResult<CursorPage<AuditLogDto>> {
        if !actor.has_capability("audit", "read") {
            return Err(ApplicationError::forbidden("missing capability audit:read"));
        }
        let limit = normalize_limit(query.limit);
        let typed_cursor = match query.cursor {
            Some(ref t) => Some(AuditLogCursor::decode(t).map_err(ApplicationError::from)?),
            None => None,
        };
        let (items, next_cursor) = self.repo.find_by_user(user_id, limit, typed_cursor).await.map_err(ApplicationError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }

    pub async fn list_by_resource(&self, actor: &AuthenticatedUser, resource_type: &str, resource_id: i64, query: ListAuditQuery) -> ApplicationResult<CursorPage<AuditLogDto>> {
        if !actor.has_capability("audit", "read") {
            return Err(ApplicationError::forbidden("missing capability audit:read"));
        }
        let limit = normalize_limit(query.limit);
        let typed_cursor = match query.cursor {
            Some(ref t) => Some(AuditLogCursor::decode(t).map_err(ApplicationError::from)?),
            None => None,
        };
        let (items, next_cursor) = self.repo.find_by_resource(resource_type, resource_id, limit, typed_cursor).await.map_err(ApplicationError::from)?;
        let dtos: Vec<_> = items.into_iter().map(Into::<AuditLogDto>::into).collect();
        Ok(CursorPage::new(dtos, next_cursor))
    }
}

fn normalize_limit(limit: u32) -> u32 {
    const DEFAULT_LIMIT: u32 = 20;
    const MAX_LIMIT: u32 = 100;

    if limit == 0 { DEFAULT_LIMIT } else { limit.min(MAX_LIMIT) }
}
