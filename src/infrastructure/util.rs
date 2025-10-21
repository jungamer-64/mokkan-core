use crate::application::ports::util::SlugGenerator;
use slug::slugify;

#[derive(Default, Clone)]
pub struct DefaultSlugGenerator;

impl SlugGenerator for DefaultSlugGenerator {
    fn slugify(&self, input: &str) -> String {
        slugify(input)
    }
}
