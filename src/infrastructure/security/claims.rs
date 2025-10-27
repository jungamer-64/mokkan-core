// src/infrastructure/security/claims.rs
use crate::application::{
    dto::AuthenticatedUser,
    error::{ApplicationError, ApplicationResult},
};
use crate::domain::user::Capability;
use chrono::DateTime;
use chrono::Utc;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn parse_claims(
    facts: Vec<biscuit_auth::builder::Fact>,
) -> ApplicationResult<AuthenticatedUser> {
    let ctx = ClaimsContext::from_facts(facts);
    build_authenticated_user(ctx)
}

fn build_authenticated_user(ctx: ClaimsContext) -> ApplicationResult<AuthenticatedUser> {
    let (user_id_i64, username, role, issued_at, expires_at) = validate_claims(&ctx)?;

    let user_id =
        crate::domain::user::UserId::new(user_id_i64).map_err(|err| ApplicationError::from(err))?;

    let mut all_caps = role.default_capabilities();
    all_caps.extend(ctx.capabilities);

    Ok(AuthenticatedUser {
        id: user_id,
        username,
        role,
        capabilities: all_caps,
        issued_at: DateTime::<Utc>::from(issued_at),
        expires_at: DateTime::<Utc>::from(expires_at),
        session_id: ctx.session_id,
        token_version: ctx.token_version,
    })
}

fn validate_claims(
    ctx: &ClaimsContext,
) -> ApplicationResult<(
    i64,
    String,
    crate::domain::user::Role,
    SystemTime,
    SystemTime,
)> {
    let user_id = ctx
        .user_id
        .ok_or_else(|| ApplicationError::unauthorized("missing user id"))?;
    let username = ctx
        .username
        .clone()
        .ok_or_else(|| ApplicationError::unauthorized("missing username"))?;
    let role = ctx
        .role
        .clone()
        .ok_or_else(|| ApplicationError::unauthorized("missing role"))?;
    let issued_at = ctx
        .issued_at
        .ok_or_else(|| ApplicationError::unauthorized("missing issued_at"))?;
    let expires_at = ctx
        .expires_at
        .ok_or_else(|| ApplicationError::unauthorized("missing expires_at"))?;

    Ok((user_id, username, role, issued_at, expires_at))
}

#[derive(Default)]
struct ClaimsContext {
    user_id: Option<i64>,
    username: Option<String>,
    role: Option<crate::domain::user::Role>,
    issued_at: Option<SystemTime>,
    expires_at: Option<SystemTime>,
    session_id: Option<String>,
    token_version: Option<u32>,
    capabilities: std::collections::HashSet<Capability>,
}

impl ClaimsContext {
    fn from_facts(facts: Vec<biscuit_auth::builder::Fact>) -> Self {
        let mut ctx = ClaimsContext::default();
        for fact in facts {
            ctx.apply_predicate(fact.predicate);
        }
        ctx
    }

    fn apply_predicate(&mut self, predicate: biscuit_auth::builder::Predicate) {
        match predicate.name.as_str() {
            "user" => self.handle_user(&predicate),
            "role" => self.handle_role(&predicate),
            "issued_at" => self.handle_issued_at(&predicate),
            "expires_at" => self.handle_expires_at(&predicate),
            "right" => self.handle_right(&predicate),
            "session" => self.handle_session(&predicate),
            _ => {}
        }
    }

    fn handle_user(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if predicate.terms.len() == 2 {
            if let biscuit_auth::builder::Term::Integer(id) = predicate.terms[0] {
                self.user_id = Some(id);
            }
            if let biscuit_auth::builder::Term::Str(name) = predicate.terms[1].clone() {
                self.username = Some(name);
            }
        }
    }

    fn handle_role(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if let Some(term) = predicate.terms.first() {
            if let biscuit_auth::builder::Term::Str(role_name) = term.clone() {
                if let Ok(parsed) = role_name.parse() {
                    self.role = Some(parsed);
                }
            }
        }
    }

    fn handle_issued_at(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if let Some(term) = predicate.terms.first() {
            if let biscuit_auth::builder::Term::Date(seconds) = term {
                self.issued_at = Some(UNIX_EPOCH + std::time::Duration::from_secs(*seconds));
            }
        }
    }

    fn handle_expires_at(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if let Some(term) = predicate.terms.first() {
            if let biscuit_auth::builder::Term::Date(seconds) = term {
                self.expires_at = Some(UNIX_EPOCH + std::time::Duration::from_secs(*seconds));
            }
        }
    }

    fn handle_right(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if predicate.terms.len() == 2 {
            if let (
                biscuit_auth::builder::Term::Str(resource),
                biscuit_auth::builder::Term::Str(action),
            ) = (predicate.terms[0].clone(), predicate.terms[1].clone())
            {
                self.capabilities.insert(Capability::new(resource, action));
            }
        }
    }

    fn handle_session(&mut self, predicate: &biscuit_auth::builder::Predicate) {
        if predicate.terms.len() == 2 {
            if let biscuit_auth::builder::Term::Str(sid) = predicate.terms[0].clone() {
                self.session_id = Some(sid);
            }
            if let biscuit_auth::builder::Term::Integer(v) = predicate.terms[1] {
                self.token_version = Some(v as u32);
            }
        }
    }
}
