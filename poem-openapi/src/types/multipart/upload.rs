use std::{
    borrow::Cow,
    fmt::{self, Debug, Formatter},
};

use poem::web::Field as PoemField;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, Error as IoError, ErrorKind},
};

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromMultipartField, ParseResult, Type},
};

/// A uploaded file for multipart.
pub struct Upload {
    file_name: Option<String>,
    content_type: Option<String>,
    file: File,
}

impl Debug for Upload {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Upload");
        if let Some(file_name) = self.file_name() {
            d.field("filename", &file_name);
        }
        if let Some(content_type) = self.content_type() {
            d.field("content_type", &content_type);
        }
        d.finish()
    }
}

impl Upload {
    /// Get the content type of the field.
    #[inline]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// The file name found in the `Content-Disposition` header.
    #[inline]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// Consumes this body object to return a [`Vec<u8>`] that contains all
    /// data.
    pub async fn into_vec(self) -> Result<Vec<u8>, IoError> {
        let mut data = Vec::new();
        self.into_async_read().read_to_end(&mut data).await?;
        Ok(data)
    }

    /// Consumes this body object to return a [`String`] that contains all data.
    pub async fn into_string(self) -> Result<String, IoError> {
        Ok(String::from_utf8(
            self.into_vec()
                .await
                .map_err(|err| IoError::new(ErrorKind::Other, err))?
                .to_vec(),
        )
        .map_err(|err| IoError::new(ErrorKind::Other, err))?)
    }

    /// Consumes this body object to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Unpin + Send + 'static {
        self.file
    }
}

impl Type for Upload {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(binary)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "binary")))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.as_raw_value().into_iter())
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Upload {
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        match field {
            Some(field) => {
                let content_type = field.content_type().map(ToString::to_string);
                let file_name = field.file_name().map(ToString::to_string);
                Ok(Self {
                    content_type,
                    file_name,
                    file: field.tempfile().await.map_err(ParseError::custom)?,
                })
            }
            None => Err(ParseError::expected_input()),
        }
    }
}
