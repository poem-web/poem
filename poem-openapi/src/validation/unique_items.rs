use std::{collections::HashSet, hash::Hash, ops::Deref};

use derive_more::Display;

use crate::{
    registry::MetaSchema,
    types::Type,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display, Default)]
#[display(fmt = "uniqueItems()")]
pub struct UniqueItems;

impl UniqueItems {
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl<T: Deref<Target = [E]>, E: Type + Eq + Hash> Validator<T> for UniqueItems {
    #[inline]
    fn check(&self, value: &T) -> bool {
        let mut set = HashSet::new();
        for item in value.deref() {
            if !set.insert(item) {
                return false;
            }
        }
        true
    }
}

impl ValidatorMeta for UniqueItems {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.unique_items = Some(true);
    }
}
