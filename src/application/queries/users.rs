use crate::{
    application::{
        dto::{AuthenticatedUser, UserProfileDto},
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::UserRepository,
};
use std::sync::Arc;

pub struct UserQueryService {
    user_repo: Arc<dyn UserRepository>,
}

impl UserQueryService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    pub async fn get_profile(
        &self,
        actor: &AuthenticatedUser,
    ) -> ApplicationResult<UserProfileDto> {
        let user = self
            .user_repo
            .find_by_id(actor.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        Ok(UserProfileDto::from_parts(user, actor))
    }
}
