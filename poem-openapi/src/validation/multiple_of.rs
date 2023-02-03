use derive_more::Display;
use num_traits::AsPrimitive;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "multipleOf({n})")]
pub struct MultipleOf {
    n: f64,
}

impl MultipleOf {
    #[inline]
    pub fn new(n: f64) -> Self {
        Self { n }
    }
}

impl<T: AsPrimitive<f64>> Validator<T> for MultipleOf {
    #[inline]
    fn check(&self, value: &T) -> bool {
        value.as_() % self.n == 0.0
    }
}

impl ValidatorMeta for MultipleOf {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.multiple_of = Some(self.n);
    }
}
