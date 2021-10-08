use std::{collections::HashSet, convert::TryFrom};

use crate::{
    body::Body,
    endpoint::Endpoint,
    http::{
        header,
        header::{HeaderName, HeaderValue},
        Method, StatusCode,
    },
    middleware::Middleware,
    request::Request,
    response::Response,
    Error, IntoResponse, Result,
};

#[derive(Debug, Default, Clone)]
struct Config {
    allow_credentials: bool,
    allow_headers: HashSet<HeaderName>,
    allow_methods: HashSet<Method>,
    allow_origins: HashSet<HeaderValue>,
    expose_headers: HashSet<HeaderName>,
    max_age: i32,
}

/// Middleware for CORS
///
/// # Example
///
/// ```
/// use poem::{http::Method, middleware::Cors};
///
/// let cors = Cors::new()
///     .allow_method(Method::GET)
///     .allow_method(Method::POST)
///     .allow_credentials(false);
/// ```
#[derive(Default)]
pub struct Cors {
    config: Config,
}

impl Cors {
    /// Creates a new `CORS` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config {
                max_age: 86400,
                ..Default::default()
            },
        }
    }

    /// Set allow credentials.
    #[must_use]
    pub fn allow_credentials(mut self, allow_credentials: bool) -> Self {
        self.config.allow_credentials = allow_credentials;
        self
    }

    /// Add an allow header.
    #[must_use]
    pub fn allow_header<T>(mut self, header: T) -> Self
    where
        HeaderName: TryFrom<T>,
    {
        let header = match <HeaderName as TryFrom<T>>::try_from(header) {
            Ok(header) => header,
            Err(_) => panic!("illegal header"),
        };
        self.config.allow_headers.insert(header);
        self
    }

    /// Add an allow method.
    #[must_use]
    pub fn allow_method<T>(mut self, method: T) -> Self
    where
        Method: TryFrom<T>,
    {
        let method = match <Method as TryFrom<T>>::try_from(method) {
            Ok(method) => method,
            Err(_) => panic!("illegal method"),
        };
        self.config.allow_methods.insert(method);
        self
    }

    /// Add an allow origin.
    #[must_use]
    pub fn allow_origin<T>(mut self, origin: T) -> Self
    where
        HeaderValue: TryFrom<T>,
    {
        let origin = match <HeaderValue as TryFrom<T>>::try_from(origin) {
            Ok(origin) => origin,
            Err(_) => panic!("illegal origin"),
        };
        self.config.allow_origins.insert(origin);
        self
    }

    /// Add an expose method.
    #[must_use]
    pub fn expose_header<T>(mut self, header: T) -> Self
    where
        HeaderName: TryFrom<T>,
    {
        let header = match <HeaderName as TryFrom<T>>::try_from(header) {
            Ok(header) => header,
            Err(_) => panic!("illegal header"),
        };
        self.config.expose_headers.insert(header);
        self
    }

    /// Set max age.
    #[must_use]
    pub fn max_age(mut self, max_age: i32) -> Self {
        self.config.max_age = max_age;
        self
    }
}

impl<E: Endpoint> Middleware<E> for Cors {
    type Output = CorsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CorsEndpoint {
            inner: ep,
            config: self.config.clone(),
        }
    }
}

/// Endpoint for Cors middleware.
pub struct CorsEndpoint<E> {
    inner: E,
    config: Config,
}

impl<E> CorsEndpoint<E> {
    fn is_valid_origin(&self, origin: &str) -> bool {
        if self.config.allow_origins.is_empty() {
            true
        } else {
            self.config.allow_origins.iter().any(|x| {
                if x == "*" {
                    return true;
                }
                x == origin
            })
        }
    }

    fn build_preflight_response(&self) -> Response {
        let mut builder = Response::builder();

        if !self.config.allow_origins.is_empty() {
            for origin in &self.config.allow_origins {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
            }
        } else {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        }

        for method in &self.config.allow_methods {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_METHODS, method.as_str());
        }

        if !self.config.allow_headers.is_empty() {
            for header in &self.config.allow_headers {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, header.clone());
            }
        } else {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*");
        }

        builder = builder.header(header::ACCESS_CONTROL_MAX_AGE, self.config.max_age);

        if self.config.allow_credentials {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        for header in &self.config.expose_headers {
            builder = builder.header(header::ACCESS_CONTROL_EXPOSE_HEADERS, header.clone());
        }

        builder.body(Body::empty())
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CorsEndpoint<E> {
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        let origin = match req.headers().get(header::ORIGIN) {
            Some(origin) => origin.to_str().map(ToString::to_string),
            None => {
                // This is not a CORS request if there is no Origin header
                return Ok(self.inner.call(req).await.into_response());
            }
        };
        let origin = origin.map_err(|_| Error::new(StatusCode::BAD_REQUEST))?;

        if !self.is_valid_origin(&origin) {
            return Err(Error::new(StatusCode::UNAUTHORIZED));
        }

        if req.method() == Method::OPTIONS {
            return Ok(self.build_preflight_response());
        }

        let mut resp = self.inner.call(req).await.into_response();

        if self.config.allow_origins.is_empty() {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            );
        } else {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_str(&origin).unwrap(),
            );
        }

        if self.config.allow_credentials {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }

        resp.headers_mut().extend(
            self.config
                .expose_headers
                .iter()
                .map(|value| (header::ACCESS_CONTROL_EXPOSE_HEADERS, value.clone().into())),
        );

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        endpoint::make_sync,
        handler,
        middleware::CookieJarManager,
        web::cookie::{Cookie, CookieJar},
        EndpointExt,
    };

    const ALLOW_ORIGIN: &str = "example.com";
    const EXPOSE_HEADER: &str = "X-My-Custom-Header";

    fn request() -> Request {
        Request::builder()
            .method(Method::OPTIONS)
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .finish()
    }

    fn get_request() -> Request {
        Request::builder()
            .method(Method::GET)
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .finish()
    }

    #[tokio::test]
    async fn preflight_request() {
        let ep = make_sync(|_| "hello").with(
            Cors::new()
                .allow_origin(ALLOW_ORIGIN)
                .allow_method(Method::GET)
                .allow_method(Method::POST)
                .allow_method(Method::OPTIONS)
                .allow_method(Method::DELETE)
                .expose_header(EXPOSE_HEADER)
                .allow_credentials(true),
        );
        let resp = ep.map_to_response().call(request()).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            ALLOW_ORIGIN
        );
        let mut allow_methods = resp
            .headers()
            .get_all(header::ACCESS_CONTROL_ALLOW_METHODS)
            .into_iter()
            .collect::<Vec<_>>();
        allow_methods.sort();
        assert_eq!(
            allow_methods,
            vec![
                HeaderValue::from_static("DELETE"),
                HeaderValue::from_static("GET"),
                HeaderValue::from_static("OPTIONS"),
                HeaderValue::from_static("POST"),
            ],
        );
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .unwrap(),
            "*"
        );
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_MAX_AGE).unwrap(),
            "86400"
        );
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .unwrap(),
            "true"
        );
    }

    #[tokio::test]
    async fn default_cors_middleware() {
        let ep = make_sync(|_| "hello").with(Cors::new());
        let resp = ep.map_to_response().call(get_request()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn custom_cors_middleware() {
        let ep = make_sync(|_| "hello").with(
            Cors::new()
                .allow_origin(ALLOW_ORIGIN)
                .allow_method(Method::GET)
                .allow_method(Method::POST)
                .allow_method(Method::OPTIONS)
                .allow_method(Method::DELETE)
                .expose_header(EXPOSE_HEADER)
                .allow_credentials(true),
        );
        let resp = ep.map_to_response().call(get_request()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            ALLOW_ORIGIN
        );
    }

    #[tokio::test]
    async fn credentials_true() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_credentials(true));
        let resp = ep.map_to_response().call(get_request()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .unwrap(),
            "true"
        );
    }

    #[tokio::test]
    async fn set_allow_origin_list() {
        let ep = make_sync(|_| "hello")
            .with(Cors::new().allow_origin("foo.com").allow_origin("bar.com"))
            .map_to_response();

        for origin in &["foo.com", "bar.com"] {
            let resp = ep
                .call(
                    Request::builder()
                        .method(Method::GET)
                        .header(header::ORIGIN, HeaderValue::from_str(origin).unwrap())
                        .finish(),
                )
                .await;

            assert_eq!(resp.status(), StatusCode::OK);
            assert_eq!(
                resp.headers()
                    .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .unwrap(),
                origin
            );
        }
    }

    #[tokio::test]
    async fn unauthorized_origin() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_origin(ALLOW_ORIGIN));
        let resp = ep
            .map_to_response()
            .call(
                Request::builder()
                    .method(Method::GET)
                    .header(header::ORIGIN, "foo.com")
                    .finish(),
            )
            .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn retain_cookies() {
        #[handler(internal)]
        async fn index(cookie_jar: &CookieJar) {
            cookie_jar.add(Cookie::new_with_str("foo", "bar"));
        }

        let ep = index
            .with(CookieJarManager)
            .with(Cors::new().allow_origin(ALLOW_ORIGIN));
        let resp = ep.map_to_response().call(get_request()).await;

        assert_eq!(resp.headers().get(header::SET_COOKIE).unwrap(), "foo=bar");
    }

    #[tokio::test]
    async fn set_cors_headers_to_error_responses() {
        let ep = make_sync(|_| Err::<(), Error>(Error::new(StatusCode::BAD_REQUEST)))
            .with(Cors::new().allow_origin(ALLOW_ORIGIN));
        let resp = ep.map_to_response().call(get_request()).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            ALLOW_ORIGIN
        );
    }

    #[tokio::test]
    async fn no_cors_requests() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_origin(ALLOW_ORIGIN));
        let resp = ep
            .map_to_response()
            .call(Request::builder().method(Method::GET).finish())
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
    }
}
