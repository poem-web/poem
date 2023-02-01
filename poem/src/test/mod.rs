//! Test utilities to test your endpoints.
//!
//! # Basic usage
//!
//! ```
//! use poem::{handler, test::TestClient, Route};
//!
//! #[handler]
//! fn index() -> &'static str {
//!     "hello"
//! }
//!
//! let app = Route::new().at("/", index);
//! let cli = TestClient::new(app);
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! // send request
//! let resp = cli.get("/").send().await;
//! // check the status code
//! resp.assert_status_is_ok();
//! // check the body string
//! resp.assert_text("hello").await;
//! # });
//! ```
//!
//! # Check the JSON response
//!
//! ```no_run
//! use poem::{handler, test::TestClient, web::Json, Route};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct MyResponse {
//!     a: i32,
//!     b: String,
//! }
//!
//! #[handler]
//! fn index() -> Json<MyResponse> {
//!     Json(MyResponse {
//!         a: 100,
//!         b: "hello".to_string(),
//!     })
//! }
//!
//! let app = Route::new().at("/", index);
//! let cli = TestClient::new(app);
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! // send request
//! let resp = cli.get("/").send().await;
//! // check the status code
//! resp.assert_status_is_ok();
//! // check the json
//! let json = resp.json().await;
//! let json_value = json.value();
//! json_value.object().get("a").assert_i64(100);
//! json_value.object().get("b").assert_string("hello");
//! # });
//! ```
//!
//! # Post multipart data
//!
//! ```ignore
//! use poem::{
//!     error::{BadRequest, Error},
//!     handler,
//!     http::StatusCode,
//!     test::{TestClient, TestForm},
//!     web::{Form, Multipart},
//!     Result, Route,
//! };
//!
//! #[handler]
//! async fn index(mut multipart: Multipart) -> Result<String> {
//!     let mut name = None;
//!     let mut value = None;
//!
//!     while let Some(field) = multipart.next_field().await? {
//!         match field.name() {
//!             Some("name") => name = Some(field.text().await?),
//!             Some("value") => {
//!                 value = Some(field.text().await?.parse::<i32>().map_err(BadRequest)?)
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     match (name, value) {
//!         (Some(name), Some(value)) => Ok(format!("{}={}", name, value)),
//!         _ => Err(Error::from_status(StatusCode::BAD_REQUEST)),
//!     }
//! }
//!
//! let app = Route::new().at("/", index);
//! let cli = TestClient::new(app);
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! // send request
//! let resp = cli
//!     .post("/")
//!     .multipart(TestForm::new().text("name", "a").text("value", "10"))
//!     .send()
//!     .await;
//! // check the status code
//! resp.assert_status_is_ok();
//! // check the body string
//! resp.assert_text("a=10").await;
//! # });
//! ```

mod client;
mod form;
mod json;
mod request_builder;
mod response;

pub use client::TestClient;
pub use form::{TestForm, TestFormField};
pub use json::{TestJson, TestJsonArray, TestJsonObject, TestJsonValue};
pub use request_builder::TestRequestBuilder;
pub use response::TestResponse;
