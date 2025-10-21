// src/application/ports/util.rs
pub trait SlugGenerator: Send + Sync {
    fn slugify(&self, input: &str) -> String;
}
