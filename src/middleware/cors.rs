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

#[derive(Default)]
struct Config {
    allow_credentials: bool,
    allow_headers: HashSet<HeaderName>,
    allow_methods: HashSet<Method>,
    allow_origins: HashSet<HeaderValue>,
    expose_headers: HashSet<HeaderName>,
    max_age: i32,
}

/// Middleware for CORS
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
    type Output = CorsImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        CorsImpl {
            inner: ep,
            config: self.config,
        }
    }
}

#[doc(hidden)]
pub struct CorsImpl<E> {
    inner: E,
    config: Config,
}

impl<E> CorsImpl<E> {
    fn is_valid_origin(&self, origin: &str) -> bool {
        self.config.allow_origins.iter().any(|x| {
            if x == "*" {
                return true;
            }
            x == origin
        })
    }

    fn build_preflight_response(&self) -> Response {
        let mut builder = Response::builder();

        for origin in &self.config.allow_origins {
            builder = builder.header(header::ORIGIN, origin.clone());
        }

        for method in &self.config.allow_methods {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_METHODS, method.as_str());
        }

        for header in &self.config.allow_headers {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, header.clone());
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
impl<E: Endpoint> Endpoint for CorsImpl<E> {
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        if !self.is_valid_origin(
            req.headers()
                .get(header::ORIGIN)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default(),
        ) {
            return Err(Error::new(StatusCode::UNAUTHORIZED));
        }

        if req.method() == Method::OPTIONS {
            return Ok(self.build_preflight_response());
        }

        let mut resp = self.inner.call(req).await.into_response();
        if !resp.is_success() {
            return Ok(resp);
        }

        resp.headers_mut().extend(
            self.config
                .allow_origins
                .iter()
                .map(|value| (header::ACCESS_CONTROL_ALLOW_ORIGIN, value.clone())),
        );

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
