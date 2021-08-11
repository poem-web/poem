use std::ops::{Deref, DerefMut};

use crate::{Error, FromRequest, Request};

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
impl<T: Clone + Send + Sync + 'static> FromRequest for Data<T> {
    async fn from_request(req: &mut Request) -> crate::Result<Self> {
        req.extensions()
            .get::<T>()
            .cloned()
            .ok_or_else(|| {
                Error::internal_server_error(anyhow::anyhow!(
                    "Data of type `{}` was not found.",
                    std::any::type_name::<T>()
                ))
            })
            .map(Data)
    }
}
