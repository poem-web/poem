///! logger
use std::time::Instant;

use crate::prelude::Endpoint;
use async_trait::async_trait;
use tracing::{Level, Span};

use super::Middleware;

///A default subscriber with Level INFO
pub fn start() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    tracing::info!("Logger started level: {}", Level::INFO);
}

///Constructs a new Logger with the given Level.See more documentation of tracing_subscriber.
pub fn start_with_level(level: Level) {
    tracing_subscriber::fmt().with_max_level(level).init();
    tracing::info!("Logger started level: {}", level);
}

///A middleware for recording requests and responses
pub struct Logger {
    span: Span,
}

impl Logger {
    ///Constructs a new Logger with the given Span.
    pub fn new(span: Span) -> Self {
        Logger { span }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger {
            span: Span::current(),
        }
    }
}

impl<E> Middleware<E> for Logger
where
    E: Endpoint,
{
    type Output = LoggerImpl<E>;
    fn transform(self, ep: E) -> Self::Output {
        LoggerImpl {
            span: self.span.clone(),
            inner: ep,
        }
    }
}

#[doc(hidden)]
struct LoggerRun;

#[doc(hidden)]
pub struct LoggerImpl<E> {
    inner: E,
    span: Span,
}

#[async_trait]
impl<E> Endpoint for LoggerImpl<E>
where
    E: Endpoint,
{
    async fn call(
        &self,
        mut req: crate::prelude::Request,
    ) -> crate::prelude::Result<crate::prelude::Response> {
        if req.extensions().get::<LoggerRun>().is_some() {
            return self.inner.call(req).await;
        }
        req.extensions_mut().insert(LoggerRun);

        let _guard = self.span.enter();
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.uri().path().to_owned();

        tracing::info!(r#type = "Request", method = ?method, path = ?path);

        let res = match self.inner.call(req).await {
            Ok(res) => {
                tracing::info!(r#type = "Response", method = ?method, path = ?path, status=?res.status(), duration = ?start.elapsed());
                res
            }
            Err(error) => {
                tracing::error!(r#type = "Response", method = ?method, path = ?path, status=?error.status(), duration = ?start.elapsed());
                error.as_response()
            }
        };

        Ok(res)
    }
}
