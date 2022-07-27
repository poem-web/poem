use std::fmt::Write;

use poem::{http::header::CONTENT_DISPOSITION, Body, IntoResponse, Response};

use crate::{
    payload::{Binary, Payload},
    registry::{MetaHeader, MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::Type,
    ApiResponse,
};

const CONTENT_DISPOSITION_DESC: &str = "Indicate if the content is expected to be displayed inline in the browser, that is, as a Web page or as part of a Web page, or as an attachment, that is downloaded and saved locally.";

/// Attachment type
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AttachmentType {
    /// Indicate it can be displayed inside the Web page, or as the Web page
    Inline,
    /// Indicate it should be downloaded; most browsers presenting a 'Save as'
    /// dialog
    Attachment,
}

impl AttachmentType {
    #[inline]
    fn as_str(&self) -> &'static str {
        match self {
            AttachmentType::Inline => "inline",
            AttachmentType::Attachment => "attachment",
        }
    }
}

/// A binary payload for download file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attachment<T> {
    data: Binary<T>,
    ty: AttachmentType,
    filename: Option<String>,
}

impl<T: Into<Body> + Send> Attachment<T> {
    /// Create an attachment with data.
    pub fn new(data: T) -> Self {
        Self {
            data: Binary(data),
            ty: AttachmentType::Attachment,
            filename: None,
        }
    }

    /// Specify the attachment. (defaults to: [`AttachmentType::Inline`])
    #[must_use]
    pub fn attachment_type(self, ty: AttachmentType) -> Self {
        Self { ty, ..self }
    }

    /// Specify the file name.
    #[must_use]
    pub fn filename(self, filename: impl Into<String>) -> Self {
        Self {
            filename: Some(filename.into()),
            ..self
        }
    }

    fn content_disposition(&self) -> String {
        let mut content_disposition = self.ty.as_str().to_string();

        if let Some(legal_filename) = self.filename.as_ref().map(|filename| {
            filename
                .replace('\\', "\\\\")
                .replace('\"', "\\\"")
                .replace('\r', "\\\r")
                .replace('\n', "\\\n")
        }) {
            _ = write!(content_disposition, "; filename=\"{}\"", legal_filename);
        }

        content_disposition
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
        let content_disposition = self.content_disposition();
        self.data
            .with_header(CONTENT_DISPOSITION, content_disposition)
            .into_response()
    }
}

impl<T: Into<Body> + Send> ApiResponse for Attachment<T> {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: Self::CONTENT_TYPE,
                    schema: Self::schema_ref(),
                }],
                headers: vec![MetaHeader {
                    name: "Content-Disposition".to_string(),
                    description: Some(CONTENT_DISPOSITION_DESC.to_string()),
                    required: true,
                    deprecated: false,
                    schema: String::schema_ref(),
                }],
            }],
        }
    }

    fn register(_registry: &mut Registry) {}
}
