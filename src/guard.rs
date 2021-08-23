//! Route match guards.

use std::borrow::Cow;

use crate::{
    http::{
        header::{self, HeaderName, HeaderValue},
        Method,
    },
    Request,
};

/// Represents a guard used for route selection.
pub trait Guard: Send + Sync + 'static {
    /// Check if request matches predicate for route selection.
    fn check(&self, req: &Request) -> bool;
}

struct MethodGuard(Method);

impl Guard for MethodGuard {
    fn check(&self, req: &Request) -> bool {
        self.0 == req.method()
    }
}

/// Predicate to matches specified HTTP method.
pub fn method(method: Method) -> impl Guard {
    MethodGuard(method)
}

/// Predicate to matches `GET` HTTP method.
pub fn get() -> impl Guard {
    MethodGuard(Method::GET)
}

/// Predicate to matches `POST` HTTP method.
pub fn post() -> impl Guard {
    MethodGuard(Method::POST)
}

/// Predicate to matches `PUT` HTTP method.
pub fn put() -> impl Guard {
    MethodGuard(Method::PUT)
}

/// Predicate to matches `DELETE` HTTP method.
pub fn delete() -> impl Guard {
    MethodGuard(Method::DELETE)
}

/// Predicate to matches `HEAD` HTTP method.
pub fn head() -> impl Guard {
    MethodGuard(Method::HEAD)
}

/// Predicate to matches `OPTIONS` HTTP method.
pub fn options() -> impl Guard {
    MethodGuard(Method::OPTIONS)
}

/// Predicate to matches `CONNECT` HTTP method.
pub fn connect() -> impl Guard {
    MethodGuard(Method::CONNECT)
}

/// Predicate to matches `PATCH` HTTP method.
pub fn patch() -> impl Guard {
    MethodGuard(Method::PATCH)
}

/// Predicate to matches `TRACE` HTTP method.
pub fn trace() -> impl Guard {
    MethodGuard(Method::TRACE)
}

struct HeaderGuard {
    name: HeaderName,
    value: HeaderValue,
}

impl Guard for HeaderGuard {
    fn check(&self, req: &Request) -> bool {
        match req.headers().get(&self.name) {
            Some(value) => value == self.value,
            None => false,
        }
    }
}

/// Predicate to matches if request contains specified header and value.
pub fn header(name: &'static str, value: &'static str) -> impl Guard {
    HeaderGuard {
        name: HeaderName::from_static(name),
        value: HeaderValue::from_static(value),
    }
}

struct HostGuard(Cow<'static, str>);

impl Guard for HostGuard {
    fn check(&self, req: &Request) -> bool {
        if let Some(value) = req.headers().get(header::HOST) {
            if value.to_str().ok() == Some(&self.0) {
                return true;
            }
        }

        req.uri().host() == Some(&self.0)
    }
}

/// Predicate to matches if request contains specified Host name.
pub fn host(host: impl Into<Cow<'static, str>>) -> impl Guard {
    HostGuard(host.into())
}

/// Guard for the [`and`](GuardExt::and) method.
pub struct GuardAnd<A, B>(A, B);

impl<A, B> Guard for GuardAnd<A, B>
where
    A: Guard,
    B: Guard,
{
    fn check(&self, req: &Request) -> bool {
        self.0.check(req) && self.1.check(req)
    }
}

/// Guard for the [`or`](GuardExt::or) method.
pub struct GuardOr<A, B>(A, B);

impl<A, B> Guard for GuardOr<A, B>
where
    A: Guard,
    B: Guard,
{
    fn check(&self, req: &Request) -> bool {
        self.0.check(req) || self.1.check(req)
    }
}

/// Extension trait for [`Guard`].
pub trait GuardExt: Guard {
    /// Perform `and` operator on two rules.
    fn and<T>(self, other: T) -> GuardAnd<Self, T>
    where
        T: Guard,
        Self: Sized,
    {
        GuardAnd(self, other)
    }

    /// Perform `or` operator on two rules.
    fn or<T>(self, other: T) -> GuardOr<Self, T>
    where
        T: Guard,
        Self: Sized,
    {
        GuardOr(self, other)
    }
}

impl<T: Guard> GuardExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{http::Uri, *};

    #[test]
    fn test_method_guard() {
        assert!(method(Method::GET).check(&Request::builder().method(Method::GET).finish()));
        assert!(method(Method::POST).check(&Request::builder().method(Method::POST).finish()));
        assert!(method(Method::PUT).check(&Request::builder().method(Method::PUT).finish()));
        assert!(method(Method::DELETE).check(&Request::builder().method(Method::DELETE).finish()));
        assert!(method(Method::HEAD).check(&Request::builder().method(Method::HEAD).finish()));
        assert!(method(Method::OPTIONS).check(&Request::builder().method(Method::OPTIONS).finish()));
        assert!(method(Method::CONNECT).check(&Request::builder().method(Method::CONNECT).finish()));
        assert!(method(Method::PATCH).check(&Request::builder().method(Method::PATCH).finish()));
        assert!(method(Method::TRACE).check(&Request::builder().method(Method::TRACE).finish()));

        assert!(get().check(&Request::builder().method(Method::GET).finish()));
        assert!(post().check(&Request::builder().method(Method::POST).finish()));
        assert!(put().check(&Request::builder().method(Method::PUT).finish()));
        assert!(delete().check(&Request::builder().method(Method::DELETE).finish()));
        assert!(head().check(&Request::builder().method(Method::HEAD).finish()));
        assert!(options().check(&Request::builder().method(Method::OPTIONS).finish()));
        assert!(connect().check(&Request::builder().method(Method::CONNECT).finish()));
        assert!(patch().check(&Request::builder().method(Method::PATCH).finish()));
        assert!(trace().check(&Request::builder().method(Method::TRACE).finish()));

        assert!(!get().check(&Request::builder().method(Method::TRACE).finish()));
        assert!(!post().check(&Request::builder().method(Method::PUT).finish()));
    }

    #[test]
    fn test_host_guard() {
        assert!(host("test.com").check(&Request::builder().header("host", "test.com").finish()));
        assert!(!host("test.com").check(&Request::builder().header("host", "abc.com").finish()));

        assert!(host("test.com").check(
            &Request::builder()
                .uri(Uri::from_static("http://test.com/abc"))
                .finish()
        ));
        assert!(!host("test.com").check(
            &Request::builder()
                .uri(Uri::from_static("http://abc.com/abc"))
                .finish()
        ));

        assert!(host("test.com").check(
            &Request::builder()
                .uri(Uri::from_static("http://test.com/abc"))
                .header("host", "abc.com")
                .finish()
        ));

        assert!(host("test.com").check(
            &Request::builder()
                .uri(Uri::from_static("http://abc.com/abc"))
                .header("host", "test.com")
                .finish()
        ));
    }

    #[test]
    fn test_header_guard() {
        assert!(header("custom-header", "true")
            .check(&Request::builder().header("custom-header", "true").finish()));
        assert!(!header("custom-header", "true")
            .check(&Request::builder().header("custom-header", "false").finish()));
        assert!(!header("custom-header", "true").check(&Request::builder().finish()));
    }

    #[test]
    fn test_handler_macro() {
        #[handler(internal, method = "get")]
        fn method_get() {}

        assert!(method_get.check(&Request::builder().method(Method::GET).finish()));
        assert!(!method_get.check(&Request::builder().method(Method::PUT).finish()));

        #[handler(internal, method = "get", host = "test.com")]
        fn method_get_host() {}

        assert!(method_get_host.check(
            &Request::builder()
                .method(Method::GET)
                .header("host", "test.com")
                .finish()
        ));
        assert!(!method_get_host.check(
            &Request::builder()
                .method(Method::PUT)
                .header("host", "test.com")
                .finish()
        ));
        assert!(!method_get_host.check(
            &Request::builder()
                .method(Method::GET)
                .header("host", "abc.com")
                .finish()
        ));

        #[handler(
            internal,
            method = "get",
            header(name = "custom-header", value = "true")
        )]
        fn method_header() {}

        assert!(method_header.check(
            &Request::builder()
                .method(Method::GET)
                .header("custom-header", "true")
                .finish()
        ));
        assert!(!method_header.check(
            &Request::builder()
                .method(Method::GET)
                .header("custom-header", "false")
                .finish()
        ));
        assert!(!method_header.check(
            &Request::builder()
                .method(Method::POST)
                .header("custom-header", "true")
                .finish()
        ));

        #[handler(
            internal,
            method = "get",
            header(name = "custom-header1", value = "true"),
            header(name = "custom-header2", value = "true")
        )]
        fn method_multi_header() {}

        assert!(method_multi_header.check(
            &Request::builder()
                .method(Method::GET)
                .header("custom-header1", "true")
                .header("custom-header2", "true")
                .finish()
        ));
        assert!(!method_multi_header.check(
            &Request::builder()
                .method(Method::GET)
                .header("custom-header1", "true")
                .finish()
        ));
        assert!(!method_multi_header.check(
            &Request::builder()
                .method(Method::POST)
                .header("custom-header1", "true")
                .header("custom-header2", "true")
                .finish()
        ));
    }
}
