//! Tera Templating Support
//!
//! # Load templates from file system using a glob
//!
//! ```no_run
//! use poem::templates::tera::TeraEngine;
//!
//! let tera = TeraEngine::default();
//! ```
//!
//! # Render a template inside an handler with some context vars
//!
//! ```
//! use poem::{
//!     ctx, handler,
//!     templates::Template,
//!     web::Path,
//! };
//!
//! #[handler]
//! fn hello(Path(name): Path<String>) -> Template<_> {
//!     Template::render("index.html.tera", &ctx! { "name": &name })
//! }
//! ```

mod middleware;
mod transformers;

pub use tera::{ Context, Tera };

pub use self::{
    middleware::{ TeraEndpoint, TeraEngine, TeraTemplate },
    transformers::filters,
};

/// Macro for constructing a Tera Context
/// ```
/// use poem::{
///     ctx, handler,
///     templates::Template,
///     web::Path,
/// };
///
/// #[handler]
/// fn hello(Path(name): Path<String>) -> Template<_> {
///     Template::render("index.html.tera", &ctx! { "name": &name })
/// }
/// ```

// todo: create common macro with common context

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
