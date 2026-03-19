use super::{UserCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{AuthenticatedUser, UserDto},
        error::ApplicationResult,
    },
    domain::user::{Role, UserId, UserUpdate},
};

pub struct GrantRoleCommand {
    pub user_id: i64,
    pub role: Role,
}

pub struct RevokeRoleCommand {
    pub user_id: i64,
}

impl UserCommandService {
    /// Grant a role to a user.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks `users:update`, the user id is
    /// invalid, or the repository update fails.
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

    /// Revoke an elevated role from a user.
    ///
    /// # Errors
    ///
    /// Returns an error if the actor lacks `users:update`, the user id is
    /// invalid, or the repository update fails.
    pub async fn revoke_role(
        &self,
        actor: &AuthenticatedUser,
        command: RevokeRoleCommand,
    ) -> ApplicationResult<UserDto> {
        ensure_capability(actor, "users", "update")?;

        let user_id = UserId::new(command.user_id)?;
        let update = UserUpdate::new(user_id).with_role(Role::Author);

        let user = self.user_repo.update(update).await?;
        Ok(user.into())
    }
}
