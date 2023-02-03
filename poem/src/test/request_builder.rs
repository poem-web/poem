use headers::{Header, HeaderMapExt};
use http::{header, header::HeaderName, Extensions, HeaderMap, HeaderValue, Method};
use serde::Serialize;
use serde_json::Value;

use crate::{
    test::{TestClient, TestForm, TestResponse},
    Body, Endpoint, Request,
};

/// A request builder for testing.
pub struct TestRequestBuilder<'a, E> {
    cli: &'a TestClient<E>,
    uri: String,
    method: Method,
    query: Vec<(String, Value)>,
    headers: HeaderMap,
    body: Body,
    extensions: Extensions,
}

impl<'a, E> TestRequestBuilder<'a, E> {
    pub(crate) fn new(cli: &'a TestClient<E>, method: Method, uri: String) -> Self {
        Self {
            cli,
            uri,
            method,
            query: Default::default(),
            headers: Default::default(),
            body: Body::empty(),
            extensions: Default::default(),
        }
    }

    /// Sets the query string for this request.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, test::TestClient, web::Query, Route};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Params {
    ///     key: String,
    ///     value: i32,
    /// }
    ///
    /// #[handler]
    /// fn index(Query(params): Query<Params>) -> String {
    ///     format!("{}={}", params.key, params.value)
    /// }
    ///
    /// let app = Route::new().at("/", index);
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli
    ///     .get("/")
    ///     .query("key", &"a")
    ///     .query("value", &10)
    ///     .send()
    ///     .await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("a=10").await;
    /// # });
    /// ```
    #[must_use]
    pub fn query(mut self, name: impl Into<String>, value: &impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.query.push((name.into(), value));
        }
        self
    }

    /// Sets the header value for this request.
    #[must_use]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = value
            .try_into()
            .map_err(|_| ())
            .expect("valid header value");
        self.headers.append(key, value);
        self
    }

    /// Inserts a typed header to this request.
    #[must_use]
    pub fn typed_header<T: Header>(mut self, header: T) -> Self {
        self.headers.typed_insert(header);
        self
    }

    /// Sets the content type for this request.
    #[must_use]
    pub fn content_type(self, content_type: impl AsRef<str>) -> Self {
        self.header(header::CONTENT_TYPE, content_type.as_ref())
    }

    /// Sets the body for this request.
    #[must_use]
    pub fn body(self, body: impl Into<Body>) -> Self {
        Self {
            body: body.into(),
            ..self
        }
    }

    /// Sets the JSON body for this request with `application/json` content
    /// type.
    #[must_use]
    pub fn body_json(self, body: &impl Serialize) -> Self {
        self.content_type("application/json")
            .body(serde_json::to_string(&body).expect("valid json"))
    }

    /// Sets the XML body for this request with `application/yaml` content
    /// type.
    #[cfg(feature = "yaml")]
    #[must_use]
    pub fn body_yaml(self, body: &impl Serialize) -> Self {
        self.content_type("application/yaml")
            .body(serde_yaml::to_string(&body).expect("valid yaml"))
    }

    /// Sets the XML body for this request with `application/xml` content
    /// type.
    #[cfg(feature = "xml")]
    #[must_use]
    pub fn body_xml(self, body: &impl Serialize) -> Self {
        self.content_type("application/xml")
            .body(quick_xml::se::to_string(&body).expect("valid xml"))
    }

    /// Sets the form data for this request with
    /// `application/x-www-form-urlencoded` content type.
    #[must_use]
    pub fn form(self, form: &impl Serialize) -> Self {
        self.content_type("application/x-www-form-urlencoded")
            .body(serde_urlencoded::to_string(form).expect("valid form data"))
    }

    /// Sets the multipart body for this request with `multipart/form-data`
    /// content type.
    #[must_use]
    pub fn multipart(self, form: TestForm) -> Self {
        self.content_type(format!("multipart/form-data; boundary={}", form.boundary()))
            .body(Body::from_async_read(form.into_async_read()))
    }

    fn make_request(self) -> Request {
        let uri = if self.query.is_empty() {
            self.uri
        } else {
            format!(
                "{}?{}",
                self.uri,
                serde_urlencoded::to_string(&self.query).unwrap()
            )
        };

        let mut req = Request::builder()
            .method(self.method)
            .uri(uri.parse().expect("valid uri"))
            .finish();
        req.headers_mut().extend(self.cli.default_headers.clone());
        req.headers_mut().extend(self.headers);
        *req.extensions_mut() = self.extensions;
        req.set_body(self.body);

        req
    }

    /// Sets the extension data for this request.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, test::TestClient, web::Data, Route};
    ///
    /// #[handler]
    /// fn index(Data(value): Data<&i32>) -> String {
    ///     value.to_string()
    /// }
    ///
    /// let app = Route::new().at("/", index);
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/").data(100i32).send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100").await;
    /// # });
    /// ```
    #[must_use]
    pub fn data<T>(mut self, data: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.extensions.insert(data);
        self
    }

    /// Send this request to endpoint to get the response.
    pub async fn send(self) -> TestResponse
    where
        E: Endpoint,
    {
        let ep = &self.cli.ep;
        let req = self.make_request();
        let resp = ep.get_response(req).await;
        TestResponse::new(resp)
    }
}
