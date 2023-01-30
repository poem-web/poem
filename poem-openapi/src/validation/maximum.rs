use derive_more::Display;
use num_traits::AsPrimitive;

use crate::{
    registry::MetaSchema,
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display(fmt = "maximum({n}, exclusive: {exclusive})")]
pub struct Maximum {
    n: f64,
    exclusive: bool,
}

impl Maximum {
    #[inline]
    pub fn new(n: f64, exclusive: bool) -> Self {
        Self { n, exclusive }
    }
}

impl<T: AsPrimitive<f64>> Validator<T> for Maximum {
    #[inline]
    fn check(&self, value: &T) -> bool {
        if self.exclusive {
            value.as_() < self.n
        } else {
            value.as_() <= self.n
        }
    }
}

impl ValidatorMeta for Maximum {
    fn update_meta(&self, meta: &mut MetaSchema) {
        meta.maximum = Some(self.n);
        if self.exclusive {
            meta.exclusive_maximum = Some(true);
        }
    }
}
