// src/application/commands/users.rs
use crate::{
    application::{
        dto::{AuthTokenDto, AuthenticatedUser, TokenSubject, UserDto},
        error::{ApplicationError, ApplicationResult},
        ports::{
            security::{PasswordHasher, TokenManager},
            time::Clock,
            session_revocation::SessionRevocationStore,
        },
    },
    domain::user::{NewUser, PasswordHash, Role, UserId, UserRepository, UserUpdate, Username},
};
use std::sync::Arc;
use uuid::Uuid;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

pub struct RegisterUserCommand {
    pub username: String,
    pub password: String,
    pub role: Option<Role>,
}

pub struct UpdateUserCommand {
    pub user_id: i64,
    pub is_active: Option<bool>,
    pub role: Option<Role>,
}

pub struct ChangePasswordCommand {
    pub user_id: i64,
    pub current_password: Option<String>,
    pub new_password: String,
}

pub struct LoginUserCommand {
    pub username: String,
    pub password: String,
}

pub struct LoginResult {
    pub token: AuthTokenDto,
    pub user: UserDto,
}

pub struct RefreshTokenCommand {
    pub token: String,
}

pub struct GrantRoleCommand {
    pub user_id: i64,
    pub role: Role,
}

pub struct RevokeRoleCommand {
    pub user_id: i64,
}

pub struct UserCommandService {
    user_repo: Arc<dyn UserRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    token_manager: Arc<dyn TokenManager>,
    session_revocation_store: Arc<dyn SessionRevocationStore>,
    clock: Arc<dyn Clock>,
}

const MIN_PASSWORD_LENGTH: usize = 12;

impl UserCommandService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        session_revocation_store: Arc<dyn SessionRevocationStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repo,
            password_hasher,
            token_manager,
            session_revocation_store,
            clock,
        }
    }

    pub async fn register(
        &self,
        actor: Option<&AuthenticatedUser>,
        command: RegisterUserCommand,
    ) -> ApplicationResult<UserDto> {
        let username = Username::new(command.username)?;
        validate_password(&command.password)?;
        let existing = self.user_repo.count().await?;
        let role = self.determine_role(existing, actor, command.role).await?;

        // simple helpers to reduce cyclomatic complexity for the top-level register method
        self.ensure_username_available(existing, &username).await?;

        let user = self
            .create_and_insert_user(username.clone(), &command.password, role)
            .await?;

        Ok(user.into())
    }

    pub async fn login(&self, command: LoginUserCommand) -> ApplicationResult<LoginResult> {
        let username = Username::new(command.username)?;
        let user = self
            .find_and_authenticate_user(username, &command.password)
            .await?;

        // Create a per-login session id so clients and server can manage per-device sessions.
        let session_id = Uuid::new_v4().to_string();

        let token = self.issue_session_tokens(&user, &session_id).await?;
        let user_dto: UserDto = user.into();

        Ok(LoginResult { token, user: user_dto })
    }

    // Helper: issue access + refresh token pair for a user/session
    async fn issue_session_tokens(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
    ) -> ApplicationResult<AuthTokenDto> {
        let capabilities = user.role.default_capabilities();

        // create and persist initial refresh nonce
        let refresh_nonce = self.create_session_refresh_nonce(session_id).await?;

        // include current min_token_version in the refresh token so server-side revocation
        // via token version can be enforced during refresh
        let min_version = self
            .session_revocation_store
            .get_min_token_version(i64::from(user.id))
            .await?
            .unwrap_or(0);

        let subject = TokenSubject {
            user_id: user.id,
            username: user.username.to_string(),
            role: user.role,
            capabilities: capabilities.clone(),
            session_id: Some(session_id.to_string()),
            token_version: None,
        };

        let mut token = self.token_manager.issue(subject).await?;

    let raw_refresh = format!("{}:{}:{}:{}", i64::from(user.id), session_id, refresh_nonce, min_version);
        let refresh_token = URL_SAFE_NO_PAD.encode(raw_refresh.as_bytes());
        token.refresh_token = Some(refresh_token);

        Ok(token)
    }

    pub async fn update_user(
        &self,
        actor: &AuthenticatedUser,
        command: UpdateUserCommand,
    ) -> ApplicationResult<UserDto> {
        ensure_capability(actor, "users", "update")?;

        let user_id = UserId::new(command.user_id)?;

        if command.is_active.is_none() && command.role.is_none() {
            return Err(ApplicationError::validation(
                "at least one field must be provided",
            ));
        }

        let mut update = UserUpdate::new(user_id);

        if let Some(is_active) = command.is_active {
            update = update.with_is_active(is_active);
        }

        if let Some(role) = command.role {
            update = update.with_role(role);
        }

        let user = self.user_repo.update(update).await?;
        Ok(user.into())
    }

    pub async fn change_password(
        &self,
        actor: &AuthenticatedUser,
        command: ChangePasswordCommand,
    ) -> ApplicationResult<()> {
        let target_id = UserId::new(command.user_id)?;

        let user = self
            .user_repo
            .find_by_id(target_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        // perform authorization and verify current password when user is changing their own password
        self.verify_change_password_self(actor, &user, command.current_password.as_deref()).await?;

        // Move validation and persistence into helper to reduce complexity
        self.validate_and_set_new_password(target_id, &command.new_password)
            .await?;

        Ok(())
    }

    pub async fn refresh_token(
        &self,
        command: RefreshTokenCommand,
    ) -> ApplicationResult<AuthTokenDto> {
        // Delegate to helpers to keep this method focused and reduce complexity
        let (user, session_id, nonce, _token_ver) = self
            .validate_and_load_user_from_refresh_token(&command.token)
            .await?;

        let new_access = self
            .perform_refresh_for_user(&user, &session_id, &nonce)
            .await?;

        Ok(new_access)
    }

    pub async fn grant_role(
        &self,
        actor: &AuthenticatedUser,
        command: GrantRoleCommand,
    ) -> ApplicationResult<UserDto> {
        ensure_capability(actor, "users", "update")?;

        let user_id = UserId::new(command.user_id)?;
        let update = UserUpdate::new(user_id).with_role(command.role);

        let user = self.user_repo.update(update).await?;
        Ok(user.into())
    }

    pub async fn revoke_role(
        &self,
        actor: &AuthenticatedUser,
        command: RevokeRoleCommand,
    ) -> ApplicationResult<UserDto> {
        ensure_capability(actor, "users", "update")?;

        let user_id = UserId::new(command.user_id)?;
        // For now, revoking means setting the role back to the default (Author)
        let update = UserUpdate::new(user_id).with_role(Role::Author);

        let user = self.user_repo.update(update).await?;
        Ok(user.into())
    }
}

impl UserCommandService {
    async fn determine_role(
        &self,
        existing: u64,
        actor: Option<&AuthenticatedUser>,
        role: Option<Role>,
    ) -> ApplicationResult<Role> {
        if existing == 0 {
            return Ok(Role::Admin);
        }
        let requester = actor.ok_or_else(|| ApplicationError::forbidden("administrative privileges are required"))?;
        ensure_capability(requester, "users", "create")?;
        Ok(role.unwrap_or(Role::Author))
    }

    async fn verify_change_password_self(
        &self,
        actor: &AuthenticatedUser,
        user: &crate::domain::user::User,
        current_password: Option<&str>,
    ) -> ApplicationResult<()> {
        let is_self = actor.id == user.id;

        if !is_self {
            ensure_capability(actor, "users", "update")?;
            return Ok(());
        }

        let current = current_password
            .ok_or_else(|| ApplicationError::validation("current password is required"))?;

        self.password_hasher
            .verify(current, user.password_hash.as_str())
            .await?;

        Ok(())
    }

    // Helper: ensure username not taken (keeps register small)
    async fn ensure_username_available(&self, existing: u64, username: &Username) -> ApplicationResult<()> {
        if existing == 0 {
            return Ok(());
        }

        if self.user_repo.find_by_username(username).await?.is_some() {
            return Err(ApplicationError::conflict("username already exists"));
        }

        Ok(())
    }

    // Helper: create and persist a user from a username/password/role
    async fn create_and_insert_user(
        &self,
        username: Username,
        password: &str,
        role: Role,
    ) -> ApplicationResult<crate::domain::user::User> {
        let hashed = self.password_hasher.hash(password).await?;
        let password_hash = PasswordHash::new(hashed)?;

        let created_at = self.clock.now();
        let new_user = NewUser::new(username, password_hash, role, created_at)?;
        let user = self.user_repo.insert(new_user).await?;

        Ok(user)
    }

    // Helper: validate password and persist the new hash
    async fn validate_and_set_new_password(&self, target_id: UserId, new_password: &str) -> ApplicationResult<()> {
        validate_password(new_password)?;

        let hashed = self.password_hasher.hash(new_password).await?;
        let password_hash = PasswordHash::new(hashed)?;

        let update = UserUpdate::new(target_id).with_password_hash(password_hash);
        self.user_repo.update(update).await?;

        Ok(())
    }

    // Helper: parse/validate refresh token, ensure session and token_version are acceptable, return user + parts
    async fn validate_and_load_user_from_refresh_token(
        &self,
        token: &str,
    ) -> ApplicationResult<(crate::domain::user::User, String, String, u32)> {
        let (user_id, session_id, nonce, token_ver_in_token) = self.parse_refresh_token_str(token).await?;

        // Ensure session has not been revoked
        if self.session_revocation_store.is_revoked(&session_id).await? {
            return Err(ApplicationError::forbidden("session revoked"));
        }

        // Load user record
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;
        // Ensure token version is not globally revoked for this user
        self.ensure_token_version_not_revoked(&user, token_ver_in_token).await?;

        Ok((user, session_id, nonce, token_ver_in_token))
    }

    async fn ensure_token_version_not_revoked(&self, user: &crate::domain::user::User, token_ver_in_token: u32) -> ApplicationResult<()> {
        if let Some(min_version) = self
            .session_revocation_store
            .get_min_token_version(i64::from(user.id))
            .await?
        {
            if token_ver_in_token < min_version {
                return Err(ApplicationError::forbidden("token version revoked"));
            }
        }

        Ok(())
    }

    // Helper: perform atomic nonce rotation and issue new access + refresh tokens
    async fn perform_refresh_for_user(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
        expected_nonce: &str,
    ) -> ApplicationResult<AuthTokenDto> {
        // Atomically rotate nonce: compare stored nonce with presented one and swap to a new one
        let new_nonce = self.rotate_session_nonce_atomic(session_id, expected_nonce).await?;
        // Build token subject and issue an access token
        let subject = self.make_token_subject(user, session_id);
        let mut new_access = self.token_manager.issue(subject).await?;

        // Build refresh token payload (async because it reads min_token_version)
        let new_refresh_token = self
            .build_refresh_token_for_user(user, session_id, &new_nonce)
            .await?;

        new_access.refresh_token = Some(new_refresh_token);

        Ok(new_access)
    }

    // Helper: construct a TokenSubject for issuing access tokens
    fn make_token_subject(&self, user: &crate::domain::user::User, session_id: &str) -> TokenSubject {
        let capabilities = user.role.default_capabilities();
        TokenSubject {
            user_id: user.id,
            username: user.username.to_string(),
            role: user.role,
            capabilities: capabilities.clone(),
            session_id: Some(session_id.to_string()),
            token_version: None,
        }
    }

    // Helper: build the encoded refresh token string for a user/session/nonce
    async fn build_refresh_token_for_user(&self, user: &crate::domain::user::User, session_id: &str, nonce: &str) -> ApplicationResult<String> {
        let current_min = self
            .session_revocation_store
            .get_min_token_version(i64::from(user.id))
            .await?
            .unwrap_or(0);

        let raw_refresh = format!("{}:{}:{}:{}", i64::from(user.id), session_id, nonce, current_min);
        let new_refresh_token = URL_SAFE_NO_PAD.encode(raw_refresh.as_bytes());
        Ok(new_refresh_token)
    }

    // Helper: create and persist a refresh nonce for a newly-created session
    async fn create_session_refresh_nonce(&self, session_id: &str) -> ApplicationResult<String> {
        let refresh_nonce = Uuid::new_v4().to_string();
        self.session_revocation_store
            .set_session_refresh_nonce(session_id, &refresh_nonce)
            .await?;
        Ok(refresh_nonce)
    }

    // Helper: parse a base64 refresh token into (UserId, session_id, nonce, token_version)
    async fn parse_refresh_token_str(
        &self,
        token: &str,
    ) -> ApplicationResult<(UserId, String, String, u32)> {
        let (user_id_part, session_id, nonce, token_ver_str) = Self::decode_refresh_token_raw(token)?;

        let uid: i64 = user_id_part
            .parse()
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let user_id = UserId::new(uid)?;

        let token_ver: u32 = token_ver_str
            .parse()
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        Ok((user_id, session_id, nonce, token_ver))
    }

    // Helper: decode and split a base64 refresh token into raw parts (uid, session, nonce, token_ver).
    fn decode_refresh_token_raw(token: &str) -> ApplicationResult<(String, String, String, String)> {
        let decoded = URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let raw = String::from_utf8(decoded)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        let parts: Vec<&str> = raw.splitn(4, ':').collect();
        if parts.len() != 4 {
            return Err(ApplicationError::validation("invalid refresh token"));
        }

        Ok((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
            parts[3].to_string(),
        ))
    }

    // Helper: validate session hasn't been revoked and nonce matches stored value
    #[allow(dead_code)]
    async fn validate_session_and_nonce(
        &self,
        session_id: &str,
        nonce: &str,
    ) -> ApplicationResult<()> {
        if self.session_revocation_store.is_revoked(session_id).await? {
            return Err(ApplicationError::forbidden("session revoked"));
        }

        let stored = self
            .session_revocation_store
            .get_session_refresh_nonce(session_id)
            .await?;

        if stored.as_deref() != Some(nonce) {
            return Err(ApplicationError::forbidden("refresh token invalid or rotated"));
        }

        Ok(())
    }

    // Helper: rotate and persist a new nonce for the given session
    #[allow(dead_code)]
    async fn rotate_session_nonce(&self, session_id: &str) -> ApplicationResult<String> {
        let new_nonce = Uuid::new_v4().to_string();
        self.session_revocation_store
            .set_session_refresh_nonce(session_id, &new_nonce)
            .await?;
        Ok(new_nonce)
    }

    // Helper: atomically rotate the session nonce only if the expected nonce matches.
    async fn rotate_session_nonce_atomic(&self, session_id: &str, expected: &str) -> ApplicationResult<String> {
        // Generate new nonce
        let new_nonce = Uuid::new_v4().to_string();

        // Attempt atomic compare-and-swap; if it fails, the presented refresh token is invalid/rotated
        let swapped = self
            .session_revocation_store
            .compare_and_swap_session_refresh_nonce(session_id, expected, &new_nonce)
            .await?;

        if !swapped {
            return Err(ApplicationError::forbidden("refresh token invalid or rotated"));
        }

        Ok(new_nonce)
    }

    // Helper: find user by username, ensure active, and verify password
    async fn find_and_authenticate_user(
        &self,
        username: Username,
        password: &str,
    ) -> ApplicationResult<crate::domain::user::User> {
        let user = self
            .user_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| ApplicationError::unauthorized("invalid credentials"))?;

        if !user.is_active {
            return Err(ApplicationError::forbidden("account is disabled"));
        }

        self.password_hasher
            .verify(password, user.password_hash.as_str())
            .await?;

        Ok(user)
    }
}

fn validate_password(password: &str) -> ApplicationResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(ApplicationError::validation(format!(
            "password must be at least {MIN_PASSWORD_LENGTH} characters"
        )));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !(has_uppercase && has_lowercase && has_digit && has_special) {
        return Err(ApplicationError::validation(
            "password must contain uppercase, lowercase, digit, and special character",
        ));
    }

    Ok(())
}

fn ensure_capability(
    user: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> ApplicationResult<()> {
    if user.has_capability(resource, action) {
        Ok(())
    } else {
        Err(ApplicationError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
