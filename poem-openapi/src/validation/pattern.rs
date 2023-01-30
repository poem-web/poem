use derive_more::Display;
use regex::Regex;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "pattern(\"{pattern}\")")]
pub struct Pattern {
    pattern: &'static str,
}

impl Pattern {
    #[inline]
    pub fn new(pattern: &'static str) -> Self {
        Self { pattern }
    }
}

impl<T: AsRef<str>> Validator<T> for Pattern {
    #[inline]
    fn check(&self, value: &T) -> bool {
        Regex::new(self.pattern).unwrap().is_match(value.as_ref())
    }
}

impl ValidatorMeta for Pattern {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.pattern = Some(self.pattern.to_string());
    }
}
