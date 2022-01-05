use std::fmt::Display;

mod max_items;
mod max_length;
mod max_properties;
mod maximum;
mod min_items;
mod min_length;
mod min_properties;
mod minimum;
mod multiple_of;
mod pattern;
mod unique_items;

pub use max_items::MaxItems;
pub use max_length::MaxLength;
pub use max_properties::MaxProperties;
pub use maximum::Maximum;
pub use min_items::MinItems;
pub use min_length::MinLength;
pub use min_properties::MinProperties;
pub use minimum::Minimum;
pub use multiple_of::MultipleOf;
pub use pattern::Pattern;
pub use unique_items::UniqueItems;

use crate::registry::MetaSchema;

pub trait Validator<T>: Display {
    fn check(&self, value: &T) -> bool;
}

pub trait ValidatorMeta {
    fn update_meta(&self, meta: &mut MetaSchema);
}
