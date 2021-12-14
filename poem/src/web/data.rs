use std::ops::Deref;

use crate::{error::GetDataError, FromRequest, Request, RequestBody, Result};

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
///     get, handler, http::StatusCode, middleware::AddData, web::Data, Endpoint, EndpointExt,
///     Request, Route,
/// };
///
/// #[handler]
/// async fn index(data: Data<&i32>) {
///     assert_eq!(*data.0, 10);
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new().at("/", get(index)).with(AddData::new(10));
/// let resp = app.call(Request::default()).await.unwrap();
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

#[async_trait::async_trait]
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
    use super::*;
    use crate::{handler, middleware::AddData, Endpoint, EndpointExt};

    #[tokio::test]
    async fn test_data_extractor() {
        #[handler(internal)]
        async fn index(value: Data<&i32>) {
            assert_eq!(value.0, &100);
        }

        let app = index.with(AddData::new(100i32));
        app.call(Request::default()).await.unwrap();
    }

    #[tokio::test]
    async fn test_data_extractor_error() {
        #[handler(internal)]
        async fn index(_value: Data<&i32>) {
            todo!()
        }

        let app = index;
        assert_eq!(
            app.call(Request::default())
                .await
                .unwrap_err()
                .downcast_ref::<GetDataError>(),
            Some(&GetDataError("i32"))
        );
    }

    #[tokio::test]
    async fn test_data_extractor_deref() {
        #[handler(internal)]
        async fn index(value: Data<&String>) {
            assert_eq!(value.to_uppercase(), "ABC");
        }

        let app = index.with(AddData::new("abc".to_string()));
        app.call(Request::default()).await.unwrap();
    }
}
