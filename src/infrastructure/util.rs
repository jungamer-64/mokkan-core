use crate::domain::article::services::SlugGenerator;
use slug::slugify;

#[derive(Default, Clone)]
pub struct DefaultSlugGenerator;

impl SlugGenerator for DefaultSlugGenerator {
    fn slugify(&self, input: &str) -> String {
        slugify(input)
    }
}
