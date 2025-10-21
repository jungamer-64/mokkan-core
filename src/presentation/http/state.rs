use crate::application::services::ApplicationServices;
use std::sync::Arc;

#[derive(Clone)]
pub struct HttpState {
    pub services: Arc<ApplicationServices>,
}
