// src/infrastructure/security/authorization_code_store.rs
use crate::application::AppResult;
use crate::application::ports::authorization_code::{Code, CodeStore};
use crate::async_support::{BoxFuture, boxed};
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

impl CodeStore for InMemoryStore {
    fn create_code(&self, code: Code) -> BoxFuture<'_, AppResult<()>> {
        boxed(async move {
            let mut guard = self.inner.lock().unwrap();
            guard.insert(code.code.clone(), code);
            drop(guard);
            Ok(())
        })
    }

    fn get_code<'a>(&'a self, code: &'a str) -> BoxFuture<'a, AppResult<Option<Code>>> {
        boxed(async move {
            let guard = self.inner.lock().unwrap();
            let found = guard.get(code).cloned();
            drop(guard);
            Ok(found)
        })
    }

    fn consume_code<'a>(&'a self, code: &'a str) -> BoxFuture<'a, AppResult<Option<Code>>> {
        boxed(async move {
            let mut guard = self.inner.lock().unwrap();
            let removed = guard.remove(code);
            drop(guard);
            Ok(removed)
        })
    }
}

#[must_use]
pub fn into_arc(store: InMemoryStore) -> Arc<dyn CodeStore> {
    Arc::new(store)
}
