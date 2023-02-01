use derive_more::Display;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "minLength({len})")]
pub struct MinLength {
    len: usize,
}

impl MinLength {
    #[inline]
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl<T: AsRef<str>> Validator<T> for MinLength {
    #[inline]
    fn check(&self, value: &T) -> bool {
        value.as_ref().len() >= self.len
    }
}

impl ValidatorMeta for MinLength {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.min_length = Some(self.len);
    }
}
