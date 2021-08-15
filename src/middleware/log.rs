//!logger

use crate::prelude::Endpoint;
use async_trait::async_trait;
use std::time::Instant;

use super::Middleware;

#[cfg(feature = "logger")]
pub use femme::LevelFilter;

/// Start logging.
#[cfg(feature = "logger")]
pub fn start() {
    femme::start();
    kv_log_macro::info!("Logger started", { level: "Info" });
}

/// Start logging with a log level.
#[cfg(feature = "logger")]
pub fn with_level(level: LevelFilter) {
    femme::with_level(level);
    kv_log_macro::info!("Logger started", { level: format!("{}", level) });
}

/// Log all incoming requests and responses.
#[derive(Debug, Default, Clone)]
pub struct Logger {
    _priv: (),
}

impl Logger {
    /// Create a new instance of `Logger`.
    #[must_use]
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl<E> Middleware<E> for Logger
where
    E: Endpoint,
{
    type Output = LoggerImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        LoggerImpl { inner: ep }
    }
}

struct LoggerRun;

#[doc(hidden)]
pub struct LoggerImpl<E> {
    inner: E,
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

        let method = req.method().to_string();
        let path = req.uri().path().to_owned();

        kv_log_macro::info!("<-- Request received", {
            method: method,
            path: path
        });

        let start = Instant::now();

        let resp = match self.inner.call(req).await {
            Ok(res) => {
                kv_log_macro::info!("--> Response sent", {
                    method: method,
                    path: path,
                    status: format!("{}", res.status()),
                    duration: format!("{:?}", start.elapsed())
                });

                res
            }
            Err(error) => {
                kv_log_macro::error!("--> Response sent", {
                    method: method,
                    path: path,
                    status: format!("{}", error.status()),
                    duration: format!("{:?}", start.elapsed())
                });

                error.as_response()
            }
        };
        Ok(resp)
    }
}
