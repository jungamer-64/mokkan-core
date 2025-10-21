// src/application/commands/users.rs
use crate::{
    application::{
        dto::{AuthTokenDto, TokenSubject, UserDto},
        error::{ApplicationError, ApplicationResult},
        ports::{
            security::{PasswordHasher, TokenManager},
            time::Clock,
        },
    },
    domain::user::{NewUser, PasswordHash, Role, UserRepository, Username},
};
use std::sync::Arc;

use crate::application::dto::AuthenticatedUser;

pub struct RegisterUserCommand {
    pub username: String,
    pub password: String,
    pub role: Option<Role>,
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
}

fn validate_password(password: &str) -> ApplicationResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        Err(ApplicationError::validation(format!(
            "password must be at least {MIN_PASSWORD_LENGTH} characters"
        )))
    } else {
        Ok(())
    }
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
