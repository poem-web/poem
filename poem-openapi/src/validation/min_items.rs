use std::ops::Deref;

use derive_more::Display;

use crate::{
    registry::MetaSchema,
    types::Type,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "minItems({len})")]
pub struct MinItems {
    len: usize,
}

impl MinItems {
    #[inline]
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl<T: Deref<Target = [E]>, E: Type> Validator<T> for MinItems {
    #[inline]
    fn check(&self, value: &T) -> bool {
        value.deref().len() >= self.len
    }
}

impl ValidatorMeta for MinItems {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.min_items = Some(self.len);
    }
}
