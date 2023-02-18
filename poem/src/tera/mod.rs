//! Tera Templating Support
//!
//! # Load templates from file system using a glob
//!
//! ```no_run
//! use poem::tera::TeraTemplating;
//!
//! let templating = TeraTemplating::from_glob("templates/**/*");
//! ```
//!
//! # Render a template inside an handler with some context vars
//!
//! ```no_run
//! use poem::{
//!     ctx, handler,
//!     tera::{Tera, TeraTemplate},
//!     web::Path,
//! };
//!
//! #[handler]
//! fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
//!     tera.render("index.html.tera", &ctx! { "name": &name })
//! }
//! ```

mod middleware;
mod transformers;

pub use tera::{Context, Tera};

pub use self::{
    middleware::{
        TeraTemplatingEndpoint, TeraTemplatingMiddleware as TeraTemplating,
        TeraTemplatingResult as TeraTemplate,
    },
    transformers::filters,
};

/// Macro for constructing a Tera Context
/// ```no_run
/// use poem::{
///     ctx, handler,
///     tera::{Tera, TeraTemplate},
///     web::Path,
/// };
/// use tera::Tera;
///
/// #[handler]
/// fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
///     tera.render("index.html.tera", &ctx! { "name": &name })
/// }
/// ```
#[macro_export]
macro_rules! ctx {
    { $( $key:literal: $value:expr ),* } => {
        {
            let mut context = ::poem::tera::Context::new();
            $(
                context.insert($key, $value);
            )*
            context
        }
    };
}
