use std::{
    io::{Error as IoError, ErrorKind},
    str::FromStr,
};

use futures_util::TryStreamExt;
use mime::Mime;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{error::ParseMultipartError, http::header, FromRequest, Request, RequestBody, Result};

/// A single field in a multipart stream.
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
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
    pub async fn bytes(self) -> Result<Vec<u8>, IoError> {
        let mut data = Vec::new();
        let mut buf = [0; 2048];
        let mut reader = self.into_async_read();
        loop {
            let sz = reader.read(&mut buf[..]).await?;
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
    pub async fn text(self) -> Result<String, IoError> {
        String::from_utf8(self.bytes().await?).map_err(|err| IoError::new(ErrorKind::Other, err))
    }

    /// Consume this field to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Send {
        tokio_util::io::StreamReader::new(
            self.0
                .map_err(|err| std::io::Error::new(ErrorKind::Other, err.to_string())),
        )
    }
}

/// An extractor that parses `multipart/form-data` requests commonly used with
/// file uploads.
///
/// # Example
///
/// ```
/// use poem::{error::Error, web::Multipart, Result};
///
/// async fn upload(mut multipart: Multipart) -> Result<()> {
///     while let Some(field) = multipart.next_field().await? {
///         let data = field.bytes().await.map_err(Error::bad_request)?;
///         println!("{} bytes", data.len());
///     }
///     Ok(())
/// }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
pub struct Multipart {
    inner: multer::Multipart<'static>,
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Multipart {
    type Error = ParseMultipartError;

    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self, Self::Error> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|err| err.to_str().ok())
            .and_then(|value| Mime::from_str(value).ok())
            .ok_or(ParseMultipartError::ContentTypeRequired)?;

        if content_type.essence_str() != mime::MULTIPART_FORM_DATA {
            return Err(ParseMultipartError::InvalidContentType(
                content_type.essence_str().to_string(),
            ));
        }

        let boundary = multer::parse_boundary(content_type.as_ref())?;
        Ok(Self {
            inner: multer::Multipart::new(
                tokio_util::io::ReaderStream::new(body.take()?.into_async_read()),
                boundary,
            ),
        })
    }
}

impl Multipart {
    /// Yields the next [`Field`] if available.
    pub async fn next_field(&mut self) -> Result<Option<Field>, ParseMultipartError> {
        match self.inner.next_field().await? {
            Some(field) => Ok(Some(Field(field))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, http::StatusCode, Endpoint};

    #[tokio::test]
    async fn test_multipart_extractor_content_type() {
        #[handler(internal)]
        async fn index(_multipart: Multipart) {
            todo!()
        }

        let mut resp = index
            .call(
                Request::builder()
                    .header("content-type", "multipart/json; boundary=X-BOUNDARY")
                    .body(()),
            )
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.take_body().into_string().await.unwrap(),
            "invalid content type `multipart/json`, expect: `application/form-data`"
        );
    }

    #[tokio::test]
    async fn test_multipart_extractor() {
        #[handler(internal)]
        async fn index(mut multipart: Multipart) {
            let field = multipart.next_field().await.unwrap().unwrap();
            assert_eq!(field.name(), Some("my_text_field"));
            assert_eq!(field.text().await.unwrap(), "abcd");

            let field = multipart.next_field().await.unwrap().unwrap();
            assert_eq!(field.name(), Some("my_file_field"));
            assert_eq!(field.file_name(), Some("a-text-file.txt"));
            assert_eq!(field.content_type(), Some("text/plain"));
            assert_eq!(
                field.text().await.unwrap(),
                "Hello world\nHello\r\nWorld\rAgain"
            );
        }

        let data = "--X-BOUNDARY\r\nContent-Disposition: form-data; name=\"my_text_field\"\r\n\r\nabcd\r\n--X-BOUNDARY\r\nContent-Disposition: form-data; name=\"my_file_field\"; filename=\"a-text-file.txt\"\r\nContent-Type: text/plain\r\n\r\nHello world\nHello\r\nWorld\rAgain\r\n--X-BOUNDARY--\r\n";
        let resp = index
            .call(
                Request::builder()
                    .header("content-type", "multipart/form-data; boundary=X-BOUNDARY")
                    .body(data),
            )
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
