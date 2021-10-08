//! Route object and DSL

mod router;
mod tree;

#[allow(unreachable_pub)]
pub use router::{
    connect, delete, get, head, options, patch, post, put, trace, Route, RouteMethod,
};
pub(crate) use tree::PathParams;
