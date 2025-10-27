use super::{UserCommandService, password::validate_password};
use crate::{
    application::{
        dto::{AuthenticatedUser, UserDto},
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::{NewUser, PasswordHash, Role, Username},
};

pub struct RegisterUserCommand {
    pub username: String,
    pub password: String,
    pub role: Option<Role>,
}

impl UserCommandService {
    pub async fn register(
        &self,
        actor: Option<&AuthenticatedUser>,
        command: RegisterUserCommand,
    ) -> ApplicationResult<UserDto> {
        let username = Username::new(command.username)?;
        validate_password(&command.password)?;
        let existing = self.user_repo.count().await?;
        let role = self.determine_role(existing, actor, command.role).await?;

        self.ensure_username_available(existing, &username).await?;

        let user = self
            .create_and_insert_user(username.clone(), &command.password, role)
            .await?;

        Ok(user.into())
    }

    async fn determine_role(
        &self,
        existing: u64,
        actor: Option<&AuthenticatedUser>,
        role: Option<Role>,
    ) -> ApplicationResult<Role> {
        if existing == 0 {
            return Ok(Role::Admin);
        }
        let requester = actor
            .ok_or_else(|| ApplicationError::forbidden("administrative privileges are required"))?;
        super::capability::ensure_capability(requester, "users", "create")?;
        Ok(role.unwrap_or(Role::Author))
    }

    async fn ensure_username_available(
        &self,
        existing: u64,
        username: &Username,
    ) -> ApplicationResult<()> {
        if existing == 0 {
            return Ok(());
        }

        if self.user_repo.find_by_username(username).await?.is_some() {
            return Err(ApplicationError::conflict("username already exists"));
        }

        Ok(())
    }

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
}
