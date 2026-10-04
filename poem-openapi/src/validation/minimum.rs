use derive_more::Display;

use crate::{
    registry::{MetaSchema, MetaSchemaNumber},
    validation::{Validator, ValidatorMeta},
};

#[derive(Display)]
#[display("minimum({n}, exclusive: {exclusive})")]
pub struct Minimum {
    n: MetaSchemaNumber,
    exclusive: bool,
}

impl Minimum {
    /// Creates a bound, preserving the precision of integer arguments.
    ///
    /// Non-finite floating-point bounds reject all values and fail schema
    /// serialization.
    #[inline]
    pub fn new(n: impl Into<MetaSchemaNumber>, exclusive: bool) -> Self {
        Self {
            n: n.into(),
            exclusive,
        }
    }
}

impl<T: Copy + Into<MetaSchemaNumber>> Validator<T> for Minimum {
    #[inline]
    fn check(&self, value: &T) -> bool {
        let value: MetaSchemaNumber = (*value).into();
        if self.exclusive {
            value > self.n
        } else {
            value >= self.n
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
