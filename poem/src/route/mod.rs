//! Route object and DSL

mod router;
mod tree;

pub use router::{
    connect, delete, get, head, options, patch, post, put, route, trace, Route, RouteMethod,
};
pub(crate) use tree::PathParams;
