use std::collections::{BTreeMap, HashMap};

use derive_more::Display;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "maxProperties({len})")]
pub struct MaxProperties {
    len: usize,
}

impl MaxProperties {
    #[inline]
    pub fn new(len: usize) -> Self {
        Self { len }
    }
}

impl<K, V> Validator<HashMap<K, V>> for MaxProperties {
    #[inline]
    fn check(&self, value: &HashMap<K, V>) -> bool {
        value.len() <= self.len
    }
}

impl<K, V> Validator<BTreeMap<K, V>> for MaxProperties {
    #[inline]
    fn check(&self, value: &BTreeMap<K, V>) -> bool {
        value.len() <= self.len
    }
}

impl ValidatorMeta for MaxProperties {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.max_properties = Some(self.len);
    }
}
