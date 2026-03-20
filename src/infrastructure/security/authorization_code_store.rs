// src/infrastructure/security/authorization_code_store.rs
use crate::application::AppResult;
use crate::application::ports::authorization_code::{Code, CodeStore};
use async_trait::async_trait;
// chrono intentionally not required in this module for the in-memory store
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
#[must_use]
pub struct InMemoryStore {
    // code -> Code
    inner: Mutex<HashMap<String, Code>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl CodeStore for InMemoryStore {
    async fn create_code(&self, code: Code) -> AppResult<()> {
        let mut guard = self.inner.lock().unwrap();
        guard.insert(code.code.clone(), code);
        drop(guard);
        Ok(())
    }

    async fn get_code(&self, code: &str) -> AppResult<Option<Code>> {
        let guard = self.inner.lock().unwrap();
        let found = guard.get(code).cloned();
        drop(guard);
        Ok(found)
    }

    async fn consume_code(&self, code: &str) -> AppResult<Option<Code>> {
        let mut guard = self.inner.lock().unwrap();
        let removed = guard.remove(code);
        drop(guard);
        Ok(removed)
    }
}

#[must_use]
pub fn into_arc(store: InMemoryStore) -> Arc<dyn CodeStore> {
    Arc::new(store)
}
