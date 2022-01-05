//! Internationalization related types.

mod args;
mod locale;
mod resources;

pub use args::I18NArgs;
pub use fluent_langneg::NegotiationStrategy;
pub use locale::Locale;
pub use resources::{I18NBundle, I18NResources, I18NResourcesBuilder};
pub use unic_langid;
