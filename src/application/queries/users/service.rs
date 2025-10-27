use std::sync::Arc;

use crate::domain::user::UserRepository;

pub struct UserQueryService {
    pub(super) user_repo: Arc<dyn UserRepository>,
}

impl UserQueryService {
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }
}
