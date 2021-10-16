//! Route object and DSL

mod internal;
mod router;
mod router_domain;
mod router_method;

pub(crate) use internal::radix_tree::PathParams;
#[allow(unreachable_pub)]
pub use router::Route;
#[allow(unreachable_pub)]
pub use router_domain::RouteDomain;
#[allow(unreachable_pub)]
pub use router_method::{
    connect, delete, get, head, options, patch, post, put, trace, RouteMethod,
};
