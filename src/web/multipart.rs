use std::io::ErrorKind;

use futures_util::TryStreamExt;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::error::{Error, Result};
use crate::http::header;
use crate::request::Request;
use crate::web::FromRequest;

/// A single field in a multipart stream.
pub struct Field(multer::Field<'static>);

impl Field {
    /// Get the content type of the field.
    #[inline]
    pub fn content_type(&self) -> Option<&str> {
        self.0.content_type().map(|mime| mime.essence_str())
    }

    /// The file name found in the `Content-Disposition` header.
    #[inline]
    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// The field name found in the `Content-Disposition` header.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.0.name()
    }

    /// Get the full data of the field as Bytes.
    pub async fn bytes(self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut buf = [0; 2048];
        let mut reader = self.into_async_read();
        loop {
            let sz = reader
                .read(&mut buf[..])
                .await
                .map_err(Error::bad_request)?;
            if sz > 0 {
                data.extend_from_slice(&buf[..sz]);
            } else {
                break;
            }
        }

        Ok(data)
    }

    /// Get the full field data as text.
    #[inline]
    pub async fn text(self) -> Result<String> {
        String::from_utf8(self.bytes().await?).map_err(Error::bad_request)
    }

    /// Consume this field to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Send {
        tokio_util::io::StreamReader::new(
            self.0
                .map_err(|err| std::io::Error::new(ErrorKind::Other, err.to_string())),
        )
    }
}

/// An extractor that parses `multipart/form-data` requests commonly used with file uploads.
///
/// # Example
///
/// ```
/// use poem::web::Multipart;
/// use poem::prelude::*;
///
/// async fn upload(mut multipart: Multipart) -> Result<()> {
///     while let Some(field) = multipart.next_field().await? {
///         let data = field.bytes().await?;
///         println!("{} bytes", data.len());
///     }
///     Ok(())
/// }
/// ```
pub struct Multipart {
    inner: multer::Multipart<'static>,
}

#[async_trait::async_trait]
impl FromRequest for Multipart {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let boundary = multer::parse_boundary(
            req.headers()
                .get(header::CONTENT_TYPE)
                .ok_or_else(|| Error::bad_request(anyhow::anyhow!("expect `Content-Type` header")))?
                .to_str()
                .map_err(Error::bad_request)?,
        )
        .map_err(Error::bad_request)?;
        Ok(Self {
            inner: multer::Multipart::new(
                tokio_util::io::ReaderStream::new(req.take_body().into_async_read()),
                boundary,
            ),
        })
    }
}

impl Multipart {
    /// Yields the next [`Field`] if available.
    pub async fn next_field(&mut self) -> Result<Option<Field>> {
        match self.inner.next_field().await.map_err(Error::bad_request)? {
            Some(field) => Ok(Some(Field(field))),
            None => Ok(None),
        }
    }
}
