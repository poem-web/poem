use derive_more::Display;
use num_traits::AsPrimitive;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "minimum({n}, exclusive: {exclusive})")]
pub struct Minimum {
    n: f64,
    exclusive: bool,
}

impl Minimum {
    #[inline]
    pub fn new(n: f64, exclusive: bool) -> Self {
        Self { n, exclusive }
    }
}

impl<T: AsPrimitive<f64>> Validator<T> for Minimum {
    #[inline]
    fn check(&self, value: &T) -> bool {
        if self.exclusive {
            value.as_() > self.n
        } else {
            value.as_() >= self.n
        }
    }
}

impl ValidatorMeta for Minimum {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.minimum = Some(self.n);
        if self.exclusive {
            meta.exclusive_minimum = Some(true);
        }
    }
}
