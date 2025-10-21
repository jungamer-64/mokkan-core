// src/application/commands/users.rs
use crate::{
    application::{
        dto::{AuthTokenDto, AuthenticatedUser, TokenSubject, UserDto},
        error::{ApplicationError, ApplicationResult},
        ports::{
            security::{PasswordHasher, TokenManager},
            time::Clock,
        },
    },
    domain::user::{NewUser, PasswordHash, Role, UserId, UserRepository, UserUpdate, Username},
};
use std::sync::Arc;

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

pub struct UserCommandService {
    user_repo: Arc<dyn UserRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    token_manager: Arc<dyn TokenManager>,
    clock: Arc<dyn Clock>,
}

const MIN_PASSWORD_LENGTH: usize = 12;

impl UserCommandService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repo,
            password_hasher,
            token_manager,
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
        let role = if existing == 0 {
            Role::Admin
        } else {
            let requester = actor.ok_or_else(|| {
                ApplicationError::forbidden("administrative privileges are required")
            })?;
            ensure_capability(requester, "users", "create")?;
            command.role.unwrap_or(Role::Author)
        };

        if existing > 0 {
            if self.user_repo.find_by_username(&username).await?.is_some() {
                return Err(ApplicationError::conflict("username already exists"));
            }
        }

        let password_hash = self.password_hasher.hash(&command.password).await?;
        let password_hash = PasswordHash::new(password_hash)?;

        let created_at = self.clock.now();
        let new_user = NewUser::new(username.clone(), password_hash, role, created_at)?;
        let user = self.user_repo.insert(new_user).await?;

        Ok(user.into())
    }

    pub async fn login(&self, command: LoginUserCommand) -> ApplicationResult<LoginResult> {
        let username = Username::new(command.username)?;

        let user = self
            .user_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| ApplicationError::unauthorized("invalid credentials"))?;

        if !user.is_active {
            return Err(ApplicationError::forbidden("account is disabled"));
        }

        self.password_hasher
            .verify(&command.password, user.password_hash.as_str())
            .await?;

        let capabilities = user.role.default_capabilities();
        let subject = TokenSubject {
            user_id: user.id,
            username: user.username.to_string(),
            role: user.role,
            capabilities: capabilities.clone(),
        };

        let token = self.token_manager.issue(subject).await?;
        let user_dto: UserDto = user.into();

        Ok(LoginResult {
            token,
            user: user_dto,
        })
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

        let is_self = actor.id == user.id;

        if !is_self {
            ensure_capability(actor, "users", "update")?;
        }

        if is_self {
            let current = command
                .current_password
                .as_deref()
                .ok_or_else(|| ApplicationError::validation("current password is required"))?;

            self.password_hasher
                .verify(current, user.password_hash.as_str())
                .await?;
        }

        validate_password(&command.new_password)?;

        let hashed = self.password_hasher.hash(&command.new_password).await?;
        let password_hash = PasswordHash::new(hashed)?;

        let update = UserUpdate::new(target_id).with_password_hash(password_hash);
        self.user_repo.update(update).await?;

        Ok(())
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
