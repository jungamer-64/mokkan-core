// src/infrastructure/security/authorization_code_store.rs
use crate::application::ApplicationResult;
use crate::application::ports::authorization_code::{AuthorizationCode, AuthorizationCodeStore};
use async_trait::async_trait;
// chrono intentionally not required in this module for the in-memory store
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct InMemoryAuthorizationCodeStore {
    // code -> AuthorizationCode
    inner: Mutex<HashMap<String, AuthorizationCode>>,
}

impl InMemoryAuthorizationCodeStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl AuthorizationCodeStore for InMemoryAuthorizationCodeStore {
    async fn create_code(&self, code: AuthorizationCode) -> ApplicationResult<()> {
        let mut guard = self.inner.lock().unwrap();
        guard.insert(code.code.clone(), code);
        Ok(())
    }

    async fn get_code(&self, code: &str) -> ApplicationResult<Option<AuthorizationCode>> {
        let guard = self.inner.lock().unwrap();
        Ok(guard.get(code).cloned())
    }

    async fn consume_code(&self, code: &str) -> ApplicationResult<Option<AuthorizationCode>> {
        let mut guard = self.inner.lock().unwrap();
        Ok(guard.remove(code))
    }
}

pub fn into_arc(store: InMemoryAuthorizationCodeStore) -> Arc<dyn AuthorizationCodeStore> {
    Arc::new(store)
}
