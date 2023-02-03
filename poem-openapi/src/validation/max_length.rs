use derive_more::Display;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "maxLength({len})")]
pub struct MaxLength {
    len: usize,
}

impl MaxLength {
    #[inline]
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl<T: AsRef<str>> Validator<T> for MaxLength {
    #[inline]
    fn check(&self, value: &T) -> bool {
        value.as_ref().len() <= self.len
    }
}

impl ValidatorMeta for MaxLength {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.max_length = Some(self.len);
    }
}
