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
//! use poem::{handler, ctx, web::Path, tera::TeraTemplate};
//! use tera::Tera;
//!
//! #[handler]
//! fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
//!     tera.render("index.html.tera", &ctx!{ "name": &name })
//! }
//! ```

mod endpoint;
mod middleware;

pub use tera::{Tera, Context};

pub use self::{
    endpoint::{TeraTemplatingEndpoint, TeraTemplatingResult as TeraTemplate},
    middleware::TeraTemplatingMiddleware as TeraTemplating
};

/// Macro for constructing a Tera Context
/// ```no_run
/// use poem::{handler, ctx, web::Path, tera::TeraTemplate};
/// use tera::Tera;
///
/// #[handler]
/// fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
///     tera.render("index.html.tera", &ctx!{ "name": &name })
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
