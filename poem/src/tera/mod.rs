//! Tera Templating Support
//!
//! # Load templates from file system using a glob
//!
//! ```no_run
//! use poem::tera::TeraTemplating;
//!
//! let templating = TeraTemplating::from_glob("templates/**/*");
//! ```

mod endpoint;
mod middleware;

pub use self::{
    endpoint::{TeraTemplatingEndpoint, TeraTemplatingResult as TeraTemplate},
    middleware::TeraTemplatingMiddleware as TeraTemplating
};