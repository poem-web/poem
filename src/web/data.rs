use std::ops::{Deref, DerefMut};

use crate::{Body, Error, FromRequest, Request, Result};

/// An extractor that can extract data from the request extension.
///
/// # Example
///
/// ```
/// use poem::{get, handler, middleware::AddData, route, web::Data, EndpointExt};
///
/// #[handler]
/// async fn index(data: Data<&i32>) {
///     assert_eq!(*data.0, 10);
/// }
///
/// let app = route().at("/", get(index)).with(AddData::new(10));
/// ```
pub struct Data<T>(pub T);

impl<T> Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Data<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: Send + Sync + 'static> FromRequest<'a> for Data<&'a T> {
    async fn from_request(req: &'a Request, _body: &mut Option<Body>) -> Result<Self> {
        req.extensions()
            .get::<T>()
            .ok_or_else(|| {
                Error::internal_server_error(anyhow::anyhow!(
                    "Data of type `{}` was not found.",
                    std::any::type_name::<T>()
                ))
            })
            .map(Data)
    }
}
