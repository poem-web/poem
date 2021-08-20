//! Route object and DSL

use std::sync::Arc;

use http::StatusCode;

use crate::{route_recognizer::Router, Endpoint, Error, Request, Response, Result};

/// Routing object
#[derive(Default)]
pub struct Route {
    router: Router<Box<dyn Endpoint>>,
}

impl Route {
    /// Add an [Endpoint] to the specified path.
    ///
    /// You can match the full path or wildcard path, and use the
    /// [`Path`](crate::web::Path) extractor to get the path parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{get, route, web::Path};
    ///
    /// #[get]
    /// async fn a() {}
    ///
    /// #[get]
    /// async fn b(Path((group, name)): Path<(String, String)>) {}
    ///
    /// #[get]
    /// async fn c(Path(path): Path<String>) {}
    ///
    /// let app = route()
    ///     // full path
    ///     .at("/a/b", a)
    ///     // capture parameters
    ///     .at("/b/:group/:name", b)
    ///     // capture tail path
    ///     .at("/c/*path", c);
    /// ```
    pub fn at(mut self, path: &str, ep: impl Endpoint) -> Self {
        self.router.add(path, Box::new(ep));
        self
    }

    /// Nest a `Endpoint` to the specified path.
    pub fn nest(mut self, path: &str, ep: impl Endpoint) -> Self {
        let ep = Arc::new(ep);
        let path = path.trim_end_matches('/');

        struct Nest<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Nest<E> {
            async fn call(&self, mut req: Request) -> Result<Response> {
                let idx = req.state().match_params.0.len() - 1;
                let (name, value) = req.state_mut().match_params.0.remove(idx);
                assert_eq!(name, "--poem-rest");
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query(value)
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await
            }
        }

        struct Root<T>(T);

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for Root<E> {
            async fn call(&self, mut req: Request) -> Result<Response> {
                req.set_uri(
                    http::uri::Builder::new()
                        .path_and_query("/")
                        .build()
                        .unwrap(),
                );
                self.0.call(req).await
            }
        }

        assert!(
            path.find('*').is_none(),
            "wildcards are not allowed in the nest path."
        );
        self.router.add(
            &format!("{}/*--poem-rest", path),
            Box::new(Nest(ep.clone())),
        );
        self.router.add(path, Box::new(Root(ep)));
        self
    }
}

/// Create a new routing object.
pub fn route() -> Route {
    Route {
        router: Default::default(),
    }
}

#[async_trait::async_trait]
impl Endpoint for Route {
    async fn call(&self, mut req: Request) -> Result<Response> {
        let m = self
            .router
            .recognize(req.uri().path())
            .map_err(|_| Error::status(StatusCode::NOT_FOUND))?;

        if !m.handler.check(&req) {
            return Err(Error::status(StatusCode::NOT_FOUND));
        }

        req.state_mut().match_params.0.extend(m.params.0);
        m.handler.call(req).await
    }
}
