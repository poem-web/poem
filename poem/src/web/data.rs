use std::ops::Deref;

use crate::{FromRequest, Request, RequestBody, Result, error::GetDataError};

/// An extractor that can extract data from the request extension.
///
/// # Errors
///
/// - [`GetDataError`]
///
/// # Example
///
/// ```
/// use poem::{
///     Endpoint, EndpointExt, Request, Route, get, handler, http::StatusCode, middleware::AddData,
///     web::Data,
/// };
///
/// #[handler]
/// async fn index(data: Data<&i32>) {
///     assert_eq!(*data.0, 10);
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new().at("/", get(index)).data(10i32);
/// let resp = app.get_response(Request::default()).await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// # });
/// ```
pub struct Data<T>(pub T);

impl<T> Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: Send + Sync + 'static> FromRequest<'a> for Data<&'a T> {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(Data(
            req.extensions()
                .get::<T>()
                .ok_or_else(|| GetDataError(std::any::type_name::<T>()))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{EndpointExt, handler, middleware::AddData, test::TestClient};

    #[tokio::test]
    async fn test_data_extractor() {
        #[handler(internal)]
        async fn index(value: Data<&i32>) {
            assert_eq!(value.0, &100);
        }

        let app = index.with(AddData::new(100i32));
        TestClient::new(app)
            .get("/")
            .send()
            .await
            .assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_data_extractor_error() {
        #[handler(internal)]
        async fn index(_value: Data<&i32>) {
            todo!()
        }

        TestClient::new(index)
            .get("/")
            .send()
            .await
            .assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_data_extractor_deref() {
        #[handler(internal)]
        async fn index(value: Data<&String>) {
            assert_eq!(value.to_uppercase(), "ABC");
        }

        TestClient::new(index.with(AddData::new("abc".to_string())))
            .get("/")
            .send()
            .await
            .assert_status_is_ok();
    }
}
