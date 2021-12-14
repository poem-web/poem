use std::str::FromStr;

use http::{uri::PathAndQuery, Uri};
use regex::Regex;

use crate::{Endpoint, Middleware, Request, Result};

/// Determines the behavior of the [`NormalizePath`] middleware.
#[derive(Debug, Clone, Copy)]
pub enum TrailingSlash {
    /// Trim trailing slashes from the end of the path.
    Trim,

    /// Only merge any present multiple trailing slashes.
    MergeOnly,

    /// Always add a trailing slash to the end of the path.
    Always,
}

impl Default for TrailingSlash {
    fn default() -> Self {
        TrailingSlash::Trim
    }
}

/// Middleware for normalizing a request's path so that routes can be matched
/// more flexibly.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{StatusCode, Uri},
///     middleware::{NormalizePath, TrailingSlash},
///     Endpoint, EndpointExt, Request, Route,
/// };
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// let app = Route::new()
///     .at("/foo/bar", get(index))
///     .with(NormalizePath::new(TrailingSlash::Trim));
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app
///     .call(
///         Request::builder()
///             .uri(Uri::from_static("/foo/bar/"))
///             .finish(),
///     )
///     .await
///     .unwrap();
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
/// # });
/// ```
pub struct NormalizePath(TrailingSlash);

impl NormalizePath {
    /// Create new `NormalizePath` middleware with the specified trailing slash
    /// style.
    pub fn new(style: TrailingSlash) -> Self {
        Self(style)
    }
}

impl<E: Endpoint> Middleware<E> for NormalizePath {
    type Output = NormalizePathEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        NormalizePathEndpoint {
            inner: ep,
            merge_slash: Regex::new("//+").unwrap(),
            style: self.0,
        }
    }
}

/// Endpoint for NormalizePath middleware.
pub struct NormalizePathEndpoint<E> {
    inner: E,
    merge_slash: Regex,
    style: TrailingSlash,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for NormalizePathEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let original_path = req
            .uri()
            .path_and_query()
            .map(|x| x.path())
            .unwrap_or_default();

        if !original_path.is_empty() {
            let path = match self.style {
                TrailingSlash::Always => format!("{}/", original_path),
                TrailingSlash::MergeOnly => original_path.to_string(),
                TrailingSlash::Trim => original_path.trim_end_matches('/').to_string(),
            };

            let path = self.merge_slash.replace_all(&path, "/");
            let path = if path.is_empty() { "/" } else { path.as_ref() };

            if path != original_path {
                let (mut parts, body) = req.into_parts();
                let mut uri_parts = parts.uri.into_parts();
                let query = uri_parts.path_and_query.as_ref().and_then(|pq| pq.query());
                let path = match query {
                    Some(query) => format!("{}?{}", path, query),
                    None => path.to_string(),
                };
                uri_parts.path_and_query = Some(PathAndQuery::from_str(&path).unwrap());

                let new_uri = Uri::from_parts(uri_parts).unwrap();
                parts.uri = new_uri;

                req = Request::from_parts(parts, body);
            }
        }

        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{endpoint::make_sync, error::NotFoundError, http::StatusCode, EndpointExt, Route};

    #[tokio::test]
    async fn trim_trailing_slashes() {
        let ep = Route::new()
            .at("/", make_sync(|_| ()))
            .at("/v1/something", make_sync(|_| ()))
            .at(
                "/v2/something",
                make_sync(|req| {
                    assert_eq!(
                        req.uri().path_and_query().as_ref().and_then(|q| q.query()),
                        Some("query=test")
                    )
                }),
            )
            .with(NormalizePath::new(TrailingSlash::Trim));

        let test_uris = [
            "/",
            "///",
            "/v1/something",
            "/v1/something/",
            "/v1/something////",
            "//v1//something",
            "//v1//something//",
            "/v2/something?query=test",
            "/v2/something/?query=test",
            "/v2/something////?query=test",
            "//v2//something?query=test",
            "//v2//something//?query=test",
        ];

        for uri in test_uris {
            let req = Request::builder().uri(Uri::from_str(uri).unwrap()).finish();
            let res = ep.call(req).await.unwrap();
            assert!(res.status().is_success(), "Failed uri: {}", uri);
        }
    }

    #[tokio::test]
    async fn trim_root_trailing_slashes_with_query() {
        let ep = Route::new()
            .at(
                "/",
                make_sync(|req| {
                    assert_eq!(
                        req.uri().path_and_query().as_ref().and_then(|q| q.query()),
                        Some("query=test")
                    )
                }),
            )
            .with(NormalizePath::new(TrailingSlash::Trim));

        let test_uris = ["/?query=test", "//?query=test", "///?query=test"];

        for uri in test_uris {
            let req = Request::builder().uri(Uri::from_str(uri).unwrap()).finish();
            let res = ep.call(req).await.unwrap();
            assert!(res.status().is_success(), "Failed uri: {}", uri);
        }
    }

    #[tokio::test]
    async fn ensure_trailing_slash() {
        let ep = Route::new()
            .at("/", make_sync(|_| ()))
            .at("/v1/something/", make_sync(|_| ()))
            .at(
                "/v2/something/",
                make_sync(|req| {
                    assert_eq!(
                        req.uri().path_and_query().as_ref().and_then(|q| q.query()),
                        Some("query=test")
                    )
                }),
            )
            .with(NormalizePath::new(TrailingSlash::Always));

        let test_uris = [
            "/",
            "///",
            "/v1/something",
            "/v1/something/",
            "/v1/something////",
            "//v1//something",
            "//v1//something//",
            "/v2/something?query=test",
            "/v2/something/?query=test",
            "/v2/something////?query=test",
            "//v2//something?query=test",
            "//v2//something//?query=test",
        ];

        for uri in test_uris {
            let req = Request::builder().uri(Uri::from_str(uri).unwrap()).finish();
            let res = ep.call(req).await.unwrap();
            assert!(res.status().is_success(), "Failed uri: {}", uri);
        }
    }

    #[tokio::test]
    async fn ensure_root_trailing_slash_with_query() {
        let ep = Route::new()
            .at(
                "/",
                make_sync(|req| {
                    assert_eq!(
                        req.uri().path_and_query().as_ref().and_then(|q| q.query()),
                        Some("query=test")
                    )
                }),
            )
            .with(NormalizePath::new(TrailingSlash::Always));

        let test_uris = ["/?query=test", "//?query=test", "///?query=test"];

        for uri in test_uris {
            let req = Request::builder().uri(Uri::from_str(uri).unwrap()).finish();
            let res = ep.call(req).await.unwrap();
            assert!(res.status().is_success(), "Failed uri: {}", uri);
        }
    }

    #[tokio::test]
    async fn keep_trailing_slash_unchanged() {
        let ep = Route::new()
            .at("/", make_sync(|_| ()))
            .at("/v1/something", make_sync(|_| ()))
            .at("/v1/", make_sync(|_| ()))
            .at(
                "/v2/something",
                make_sync(|req| {
                    assert_eq!(
                        req.uri().path_and_query().as_ref().and_then(|q| q.query()),
                        Some("query=test")
                    )
                }),
            )
            .with(NormalizePath::new(TrailingSlash::MergeOnly));

        let test_uris = [
            ("/", true), // root paths should still work
            ("/?query=test", true),
            ("///", true),
            ("/v1/something////", false),
            ("/v1/something/", false),
            ("//v1//something", true),
            ("/v1/", true),
            ("/v1", false),
            ("/v1////", true),
            ("//v1//", true),
            ("///v1", false),
            ("/v2/something?query=test", true),
            ("/v2/something/?query=test", false),
            ("/v2/something//?query=test", false),
            ("//v2//something?query=test", true),
        ];

        for (uri, success) in test_uris {
            let req = Request::builder().uri(Uri::from_str(uri).unwrap()).finish();
            let res = ep.call(req).await;

            if success {
                assert_eq!(res.unwrap().status(), StatusCode::OK, "Failed uri: {}", uri);
            } else {
                assert!(
                    res.unwrap_err().is::<NotFoundError>(),
                    "Failed uri: {}",
                    uri
                );
            }
        }
    }
}
