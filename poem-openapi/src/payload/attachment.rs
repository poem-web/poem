use poem::{http::HeaderValue, Body, IntoResponse, Response};

use crate::{
    payload::{Binary, Payload},
    registry::{MetaResponses, MetaSchemaRef, Registry},
    ApiResponse,
};

/// A binary payload for download file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attachment<T> {
    data: Binary<T>,
    filename: Option<String>,
}

impl<T: Into<Body> + Send> Attachment<T> {
    /// Create an attachment with data.
    pub fn new(data: T) -> Self {
        Self {
            data: Binary(data),
            filename: None,
        }
    }

    /// Specify the file name.
    pub fn filename(self, filename: impl Into<String>) -> Self {
        Self {
            filename: Some(filename.into()),
            ..self
        }
    }
}

impl<T: Into<Body> + Send> Payload for Attachment<T> {
    const CONTENT_TYPE: &'static str = Binary::<T>::CONTENT_TYPE;

    fn schema_ref() -> MetaSchemaRef {
        Binary::<T>::schema_ref()
    }
}

impl<T: Into<Body> + Send> IntoResponse for Attachment<T> {
    fn into_response(self) -> Response {
        let mut resp = self.data.into_response();

        if let Some(header_value) = self.filename.as_ref().and_then(|filename| {
            HeaderValue::from_str(&format!("attachment; filename={}", filename)).ok()
        }) {
            resp.headers_mut()
                .insert("Content-Disposition", header_value);
        }

        resp
    }
}

impl<T: Into<Body> + Send> ApiResponse for Attachment<T> {
    fn meta() -> MetaResponses {
        Binary::<T>::meta()
    }

    fn register(_registry: &mut Registry) {}
}
