//! Route object and DSL

mod internal;
mod router;
mod router_domain;
mod router_method;
mod router_scheme;

pub(crate) use internal::radix_tree::PathParams;
#[allow(unreachable_pub)]
pub use router::{PathPattern, Route};
#[allow(unreachable_pub)]
pub use router_domain::RouteDomain;
#[allow(unreachable_pub)]
pub use router_method::{
    connect, delete, get, head, options, patch, post, put, trace, RouteMethod,
};
#[allow(unreachable_pub)]
pub use router_scheme::RouteScheme;

use crate::error::RouteError;

pub(crate) fn check_result<T>(res: Result<T, RouteError>) -> T {
    match res {
        Ok(value) => value,
        Err(RouteError::InvalidPath(path)) => panic!("invalid path: {path}"),
        Err(RouteError::Duplicate(path)) => panic!("duplicate path: {path}"),
        Err(RouteError::InvalidRegex { path, regex }) => {
            panic!("invalid regex in path: {path} `{regex}`")
        }
    }
}
