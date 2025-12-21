use crate::{Endpoint, Middleware, Request, Result};

/// Middleware for adding any data to a request.
pub struct AddData<T> {
    value: T,
}

impl<T: Clone + Send + Sync + 'static> AddData<T> {
    /// Create new `AddData` middleware with any value.
    ///
    /// # Using with Trait Objects
    ///
    /// When using trait objects, you must explicitly coerce the value to the
    /// trait object type before passing it to this method. See the
    /// [`Data`](crate::web::Data) extractor documentation for details.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use poem::{Endpoint, EndpointExt, handler, middleware::AddData, test::TestClient, web::Data};
    ///
    /// trait Service: Send + Sync {
    ///     fn name(&self) -> &str;
    /// }
    ///
    /// struct MyService;
    /// impl Service for MyService {
    ///     fn name(&self) -> &str { "my_service" }
    /// }
    ///
    /// #[handler]
    /// async fn index(svc: Data<&Arc<dyn Service>>) -> String {
    ///     svc.name().to_string()
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// // Key: coerce to Arc<dyn Service> before passing to AddData::new
    /// let svc: Arc<dyn Service> = Arc::new(MyService);
    /// let cli = TestClient::new(index.with(AddData::new(svc)));
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("my_service").await;
    /// # });
    /// ```
    pub fn new(value: T) -> Self {
        AddData { value }
    }
}

impl<E, T> Middleware<E> for AddData<T>
where
    E: Endpoint,
    T: Clone + Send + Sync + 'static,
{
    type Output = AddDataEndpoint<E, T>;

    fn transform(&self, ep: E) -> Self::Output {
        AddDataEndpoint {
            inner: ep,
            value: self.value.clone(),
        }
    }
}

/// Endpoint for the AddData middleware.
pub struct AddDataEndpoint<E, T> {
    inner: E,
    value: T,
}

impl<E, T> Endpoint for AddDataEndpoint<E, T>
where
    E: Endpoint,
    T: Clone + Send + Sync + 'static,
{
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        req.extensions_mut().insert(self.value.clone());
        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{EndpointExt, handler, test::TestClient, web::Data};

    #[tokio::test]
    async fn test_add_data() {
        #[handler(internal)]
        async fn index(req: &Request) {
            assert_eq!(req.extensions().get::<i32>(), Some(&100));
        }

        let cli = TestClient::new(index.with(AddData::new(100i32)));
        cli.get("/").send().await.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_add_data_trait_object_with_arc() {
        // Demonstrates how to use trait objects with Data extractor.
        // Since Box<dyn Trait> doesn't implement Clone, we use Arc<dyn Trait>.
        trait Database: Send + Sync {
            fn name(&self) -> &str;
        }

        struct PostgresDb;
        impl Database for PostgresDb {
            fn name(&self) -> &str {
                "postgres"
            }
        }

        #[handler(internal)]
        async fn index(db: Data<&Arc<dyn Database>>) -> String {
            db.name().to_string()
        }

        // Key: explicitly coerce to Arc<dyn Database> before calling .data()
        let db: Arc<dyn Database> = Arc::new(PostgresDb);
        let cli = TestClient::new(index.data(db));
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("postgres").await;
    }

    #[tokio::test]
    async fn test_add_data_middleware_trait_object() {
        // Demonstrates using AddData middleware directly with trait objects.
        trait Service: Send + Sync {
            fn version(&self) -> u32;
        }

        struct MyService;
        impl Service for MyService {
            fn version(&self) -> u32 {
                42
            }
        }

        #[handler(internal)]
        async fn index(svc: Data<&Arc<dyn Service>>) -> String {
            svc.version().to_string()
        }

        // Explicitly coerce to trait object type, then use AddData::new
        let svc: Arc<dyn Service> = Arc::new(MyService);
        let cli = TestClient::new(index.with(AddData::new(svc)));
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("42").await;
    }
}
