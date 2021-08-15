use tracing::Span;
use yansi::Paint;

use crate::prelude::Endpoint;
use async_trait::async_trait;
use tracing::{field::ValueSet, Metadata};

use super::Middleware;

/// A middleware that records requests and responses
#[derive(Debug)]
pub struct Logger {
    span: Span,
}

impl Logger {
    /// Constructs a new Logger with the given span.
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

    fn transform(&self, ep: E) -> Self::Output {
        LoggerImpl {
            inner: ep,
            span: self.span.to_owned(),
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

        let start = std::time::Instant::now();
        let _guard = self.span.enter();

        let method = req.method().to_owned();
        let path = req.uri().path().to_owned();

        let resp = match self.inner.call(req).await {
            Ok(resp) => {
                let status_text = format!("{}", Paint::green(resp.status()));
                let elapsed_text = format!("{:?}", Paint::blue(start.elapsed()));

                let print = format!(
                    "{} {} {} {}",
                    Paint::green(method),
                    Paint::blue(path),
                    status_text,
                    elapsed_text
                );
                tracing::info!("{}", print);
                resp
            }
            Err(err) => {
                let status_text = format!("{}", Paint::red(err.status()));
                let elapsed_text = format!("{:?}", Paint::blue(start.elapsed()));

                let print = format!(
                    "{} {} {} {}",
                    Paint::green(method),
                    Paint::blue(path),
                    status_text,
                    elapsed_text
                );
                tracing::error!("{}", print);
                err.as_response()
            }
        };

        Ok(resp)
    }
}
