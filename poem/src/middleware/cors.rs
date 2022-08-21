use std::{collections::HashSet, str::FromStr, sync::Arc};

use headers::{
    AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlExposeHeaders, HeaderMapExt,
};

use crate::{
    endpoint::Endpoint,
    error::CorsError,
    http::{
        header,
        header::{HeaderName, HeaderValue},
        Method,
    },
    middleware::Middleware,
    request::Request,
    response::Response,
    IntoResponse, Result,
};

/// Middleware for CORS
///
/// # Errors
///
/// - [`CorsError`]
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
#[allow(clippy::type_complexity)]
pub struct Cors {
    allow_credentials: bool,
    allow_origins: HashSet<HeaderValue>,
    allow_origins_fn: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
    allow_headers: HashSet<HeaderName>,
    allow_methods: HashSet<Method>,
    expose_headers: HashSet<HeaderName>,
    max_age: i32,
}

impl Cors {
    /// Creates a new `CORS` middleware.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_age: 86400,
            ..Default::default()
        }
    }

    /// Set allow credentials.
    #[must_use]
    pub fn allow_credentials(mut self, allow_credentials: bool) -> Self {
        self.allow_credentials = allow_credentials;
        self
    }

    /// Add an allow header.
    ///
    /// NOTE: Default is allow any header.
    #[must_use]
    pub fn allow_header<T>(mut self, header: T) -> Self
    where
        HeaderName: TryFrom<T>,
    {
        let header = match <HeaderName as TryFrom<T>>::try_from(header) {
            Ok(header) => header,
            Err(_) => panic!("illegal header"),
        };
        self.allow_headers.insert(header);
        self
    }

    /// Add many allow headers.
    #[must_use]
    pub fn allow_headers<I, T>(self, headers: I) -> Self
    where
        I: IntoIterator<Item = T>,
        HeaderName: TryFrom<T>,
    {
        headers
            .into_iter()
            .fold(self, |cors, header| cors.allow_header(header))
    }

    /// Add an allow method.
    ///
    /// NOTE: Default is allow any method.
    #[must_use]
    pub fn allow_method<T>(mut self, method: T) -> Self
    where
        Method: TryFrom<T>,
    {
        let method = match <Method as TryFrom<T>>::try_from(method) {
            Ok(method) => method,
            Err(_) => panic!("illegal method"),
        };
        self.allow_methods.insert(method);
        self
    }

    /// Add many allow methods.
    #[must_use]
    pub fn allow_methods<I, T>(self, methods: I) -> Self
    where
        I: IntoIterator<Item = T>,
        Method: TryFrom<T>,
    {
        methods
            .into_iter()
            .fold(self, |cors, method| cors.allow_method(method))
    }

    /// Add an allow origin.
    ///
    /// NOTE: Default is allow any origin.
    #[must_use]
    pub fn allow_origin<T>(mut self, origin: T) -> Self
    where
        HeaderValue: TryFrom<T>,
    {
        let origin = match <HeaderValue as TryFrom<T>>::try_from(origin) {
            Ok(origin) => origin,
            Err(_) => panic!("illegal origin"),
        };
        self.allow_origins.insert(origin);
        self
    }

    /// Add many allow origins.
    #[must_use]
    pub fn allow_origins<I, T>(self, origins: I) -> Self
    where
        I: IntoIterator<Item = T>,
        HeaderValue: TryFrom<T>,
    {
        origins
            .into_iter()
            .fold(self, |cors, origin| cors.allow_origin(origin))
    }

    /// Determinate allowed origins by processing requests which didn’t match
    /// any origins specified in the `allow_origin`.
    ///
    /// This function will receive the `Origin` header, which can be used to
    /// determine whether to allow the request.
    #[must_use]
    pub fn allow_origins_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.allow_origins_fn = Some(Arc::new(f));
        self
    }

    /// Add an expose header.
    #[must_use]
    pub fn expose_header<T>(mut self, header: T) -> Self
    where
        HeaderName: TryFrom<T>,
    {
        let header = match <HeaderName as TryFrom<T>>::try_from(header) {
            Ok(header) => header,
            Err(_) => panic!("illegal header"),
        };
        self.expose_headers.insert(header);
        self
    }

    /// Add many expose headers.
    #[must_use]
    pub fn expose_headers<I, T>(self, headers: I) -> Self
    where
        I: IntoIterator<Item = T>,
        HeaderName: TryFrom<T>,
    {
        headers
            .into_iter()
            .fold(self, |cors, header| cors.expose_header(header))
    }

    /// Set max age.
    #[must_use]
    pub fn max_age(mut self, max_age: i32) -> Self {
        self.max_age = max_age;
        self
    }
}

impl<E: Endpoint> Middleware<E> for Cors {
    type Output = CorsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CorsEndpoint {
            inner: ep,
            allow_credentials: self.allow_credentials,
            allow_origins: self.allow_origins.clone(),
            allow_origins_fn: self.allow_origins_fn.clone(),
            allow_headers: self.allow_headers.clone(),
            allow_methods: self.allow_methods.clone(),
            expose_headers: self.expose_headers.clone(),
            allow_headers_header: self.allow_headers.clone().into_iter().collect(),
            allow_methods_header: self.allow_methods.clone().into_iter().collect(),
            expose_headers_header: self.expose_headers.clone().into_iter().collect(),
            max_age: self.max_age,
        }
    }
}

/// Endpoint for Cors middleware.
#[allow(clippy::type_complexity)]
pub struct CorsEndpoint<E> {
    inner: E,
    allow_credentials: bool,
    allow_origins: HashSet<HeaderValue>,
    allow_origins_fn: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
    allow_headers: HashSet<HeaderName>,
    allow_methods: HashSet<Method>,
    expose_headers: HashSet<HeaderName>,
    allow_headers_header: AccessControlAllowHeaders,
    allow_methods_header: AccessControlAllowMethods,
    expose_headers_header: AccessControlExposeHeaders,
    max_age: i32,
}

impl<E: Endpoint> CorsEndpoint<E> {
    fn is_valid_origin(&self, origin: &HeaderValue) -> (bool, bool) {
        if self.allow_origins.contains(origin) {
            return (true, false);
        }

        if let Some(allow_origins_fn) = &self.allow_origins_fn {
            if let Ok(origin) = origin.to_str() {
                if allow_origins_fn(origin) {
                    return (true, true);
                }
            }
        }

        (
            self.allow_origins.is_empty() && self.allow_origins_fn.is_none(),
            true,
        )
    }

    fn build_preflight_response(
        &self,
        origin: &HeaderValue,
        request_headers: Option<&HeaderValue>,
    ) -> Response {
        let mut builder = Response::builder()
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .typed_header(self.expose_headers_header.clone())
            .header(header::ACCESS_CONTROL_MAX_AGE, self.max_age);

        if self.allow_methods.is_empty() {
            builder = builder.typed_header(
                [
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::HEAD,
                    Method::OPTIONS,
                    Method::CONNECT,
                    Method::PATCH,
                    Method::TRACE,
                ]
                .iter()
                .cloned()
                .collect::<AccessControlAllowMethods>(),
            );
        } else {
            builder = builder.typed_header(self.allow_methods_header.clone());
        }

        if self.allow_headers.is_empty() {
            if let Some(request_headers) = request_headers {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, request_headers);
            } else {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*");
            }
        } else {
            builder = builder.typed_header(self.allow_headers_header.clone());
        }

        if self.allow_credentials {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        builder.body(())
    }

    fn check_allow_headers<'a>(&self, req: &'a Request) -> (bool, Option<&'a HeaderValue>) {
        let mut allow_headers = true;

        let request_headers = if let Some(request_header) =
            req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS)
        {
            if !self.allow_headers.is_empty() {
                allow_headers = false;
                if let Ok(s) = request_header.to_str() {
                    for header in s.split(',') {
                        if let Ok(header) = HeaderName::from_str(header.trim()) {
                            if self.allow_headers.contains(&header) {
                                allow_headers = true;
                                break;
                            }
                        }
                    }
                }
            }
            Some(request_header)
        } else {
            None
        };

        (allow_headers, request_headers)
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for CorsEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let origin = match req.headers().get(header::ORIGIN) {
            Some(origin) => origin.clone(),
            None => {
                // This is not a CORS request if there is no Origin header
                return self.inner.call(req).await.map(IntoResponse::into_response);
            }
        };

        let (origin_is_allow, vary_header) = self.is_valid_origin(&origin);
        if !origin_is_allow {
            return Err(CorsError::OriginNotAllowed.into());
        }

        if req.method() == Method::OPTIONS {
            let allow_method = req
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_METHOD)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Method>().ok())
                .map(|method| {
                    if self.allow_methods.is_empty() {
                        true
                    } else {
                        self.allow_methods.contains(&method)
                    }
                });
            if !matches!(allow_method, Some(true)) {
                return Err(CorsError::MethodNotAllowed.into());
            }

            let (allow_headers, request_headers) = self.check_allow_headers(&req);

            if !allow_headers {
                return Err(CorsError::HeadersNotAllowed.into());
            }

            return Ok(self.build_preflight_response(&origin, request_headers));
        }

        let mut resp = self.inner.get_response(req).await;

        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);

        if self.allow_credentials {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }

        if !self.expose_headers.is_empty() {
            resp.headers_mut()
                .typed_insert(self.expose_headers_header.clone());
        }

        if vary_header {
            resp.headers_mut()
                .insert(header::VARY, HeaderValue::from_static("Origin"));
        }

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{
        endpoint::make_sync,
        test::{TestClient, TestRequestBuilder},
        EndpointExt, Error,
    };

    const ALLOW_ORIGIN: &str = "https://example.com";
    const ALLOW_HEADER: &str = "X-Token";
    const EXPOSE_HEADER: &str = "X-My-Custom-Header";

    fn cors() -> Cors {
        Cors::new()
            .allow_origin(ALLOW_ORIGIN)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::DELETE])
            .allow_header(ALLOW_HEADER)
            .expose_header(EXPOSE_HEADER)
            .allow_credentials(true)
    }

    fn opt_request<T: Endpoint>(cli: &TestClient<T>) -> TestRequestBuilder<'_, T> {
        cli.options("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Token")
    }

    fn get_request<T: Endpoint>(cli: &TestClient<T>) -> TestRequestBuilder<'_, T> {
        cli.get("/").header(header::ORIGIN, ALLOW_ORIGIN)
    }

    #[tokio::test]
    async fn preflight_request() {
        let ep = make_sync(|_| "hello").with(cors());
        let cli = TestClient::new(ep);

        let resp = opt_request(&cli).send().await;

        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header_csv(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            ["DELETE", "GET", "OPTIONS", "POST"],
        );
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_HEADERS, "x-token");
        resp.assert_header(header::ACCESS_CONTROL_EXPOSE_HEADERS, "x-my-custom-header");
        resp.assert_header(header::ACCESS_CONTROL_MAX_AGE, "86400");
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
    }

    #[tokio::test]
    async fn default_cors() {
        let ep = make_sync(|_| "hello").with(Cors::new());
        let cli = TestClient::new(ep);

        let resp = cli
            .options("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Token")
            .send()
            .await;

        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header_csv(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            [
                "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "CONNECT", "PATCH", "TRACE",
            ],
        );
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_HEADERS, "X-Token");
        resp.assert_header(header::ACCESS_CONTROL_MAX_AGE, "86400");

        let resp = cli
            .get("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header(header::VARY, "Origin");
    }

    #[tokio::test]
    async fn allow_origins_fn_1() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_origins_fn(|_| true));
        let cli = TestClient::new(ep);

        let resp = cli
            .get("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header(header::VARY, "Origin");
    }

    #[tokio::test]
    async fn allow_origins_fn_2() {
        let ep = make_sync(|_| "hello").with(
            Cors::new()
                .allow_origin(ALLOW_ORIGIN)
                .allow_origins_fn(|_| true),
        );
        let cli = TestClient::new(ep);

        let resp = cli
            .get("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header_is_not_exist(header::VARY);

        let resp = cli
            .get("/")
            .header(header::ORIGIN, "https://abc.com")
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "https://abc.com");
        resp.assert_header(header::VARY, "Origin");
    }

    #[tokio::test]
    async fn allow_origins_fn_3() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_origins_fn(|_| false));
        let cli = TestClient::new(ep);

        let resp = cli
            .get("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .send()
            .await;
        resp.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn default_cors_middleware() {
        let ep = make_sync(|_| "hello").with(Cors::new());
        let cli = TestClient::new(ep);

        let resp = get_request(&cli).send().await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "https://example.com");
    }

    #[tokio::test]
    async fn unauthorized_origin() {
        let ep = make_sync(|_| "hello").with(cors());
        let cli = TestClient::new(ep);

        let resp = cli
            .get("/")
            .header(header::ORIGIN, "https://foo.com")
            .send()
            .await;
        resp.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn unauthorized_options() {
        let ep = make_sync(|_| "hello").with(cors());
        let cli = TestClient::new(ep);

        cli.options("/")
            .header(header::ORIGIN, "https://abc.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Token")
            .send()
            .await
            .assert_status(StatusCode::FORBIDDEN);

        cli.options("/")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "TRACE")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Token")
            .send()
            .await
            .assert_status(StatusCode::FORBIDDEN);

        cli.options("/")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Abc")
            .send()
            .await
            .assert_status(StatusCode::FORBIDDEN);

        cli.options("/")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Token")
            .send()
            .await
            .assert_status_is_ok();
    }

    #[cfg(feature = "cookie")]
    #[tokio::test]
    async fn retain_cookies() {
        use crate::{
            handler,
            middleware::CookieJarManager,
            web::cookie::{Cookie, CookieJar},
        };

        #[handler(internal)]
        async fn index(cookie_jar: &CookieJar) {
            cookie_jar.add(Cookie::new_with_str("foo", "bar"));
        }

        let ep = index.with(CookieJarManager::new()).with(cors());
        let cli = TestClient::new(ep);

        let resp = get_request(&cli).send().await;
        resp.assert_status_is_ok();
        resp.assert_header(header::SET_COOKIE, "foo=bar");
    }

    #[tokio::test]
    async fn set_cors_headers_to_error_responses() {
        let ep =
            make_sync(|_| Err::<(), _>(Error::from_status(StatusCode::BAD_REQUEST))).with(cors());
        let cli = TestClient::new(ep);

        let resp = get_request(&cli).send().await;
        resp.assert_status(StatusCode::BAD_REQUEST);
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
        resp.assert_header(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            EXPOSE_HEADER.to_lowercase(),
        );
    }

    #[tokio::test]
    async fn no_cors_requests() {
        let ep = make_sync(|_| "hello").with(Cors::new().allow_origin(ALLOW_ORIGIN));
        let cli = TestClient::new(ep);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_header_is_not_exist(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    }

    #[tokio::test]
    async fn allow_all_access_control_allow_headers_should_return_with_request_headers() {
        let ep = make_sync(|_| "hello").with(
            Cors::new()
                .allow_origin(ALLOW_ORIGIN)
                .allow_method(Method::GET),
        );
        let cli = TestClient::new(ep);

        let resp = cli
            .options("/")
            .header(header::ORIGIN, ALLOW_ORIGIN)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
            .send()
            .await;
        resp.assert_status_is_ok();
        resp.assert_header(header::ACCESS_CONTROL_ALLOW_HEADERS, "content-type");
    }
}
