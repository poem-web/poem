use std::ops::Deref;

use derive_more::Display;

use crate::{
    registry::MetaSchema,
    types::Type,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "maxItems({len})")]
pub struct MaxItems {
    len: usize,
}

impl MaxItems {
    #[inline]
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl<T: Deref<Target = [E]>, E: Type> Validator<T> for MaxItems {
    #[inline]
    fn check(&self, value: &T) -> bool {
        value.deref().len() <= self.len
    }
}

impl ValidatorMeta for MaxItems {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.max_items = Some(self.len);
    }
}
