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
//! use poem::{web::Path, tera::TeraTemplate};
//! use tera::Tera;
//!
//! #[handler]
//! fn hello(Path(name): Path<String>, tera: Tera) -> TeraTemplate {
//!     let mut context = Context::new();
//!     context.insert("name", &name);
//!     tera.render("index.html.tera", &context)
//! }
//! ```

mod endpoint;
mod middleware;

pub use tera::{Tera, Context};

pub use self::{
    endpoint::{TeraTemplatingEndpoint, TeraTemplatingResult as TeraTemplate},
    middleware::TeraTemplatingMiddleware as TeraTemplating
};