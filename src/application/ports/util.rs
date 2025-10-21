pub trait SlugGenerator: Send + Sync {
    fn slugify(&self, input: &str) -> String;
}
