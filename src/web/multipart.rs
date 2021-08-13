use std::io::ErrorKind;

use futures_util::TryStreamExt;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{Error, FromRequest, HeaderName, Request, Result};

pub struct Multipart {
    inner: multer::Multipart<'static>,
}

#[async_trait::async_trait]
impl FromRequest for Multipart {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let boundary = multer::parse_boundary(
            req.headers()
                .get(HeaderName::CONTENT_TYPE)
                .ok_or_else(|| Error::bad_request(anyhow::anyhow!("expect `Content-Type` header")))?
                .to_str()?,
        )
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid `Content-Type`")))?;
        Ok(Self {
            inner: multer::Multipart::new(
                tokio_util::io::ReaderStream::new(req.take_body().into_async_read()),
                boundary,
            ),
        })
    }
}

impl Multipart {
    pub async fn next_field(&mut self) -> Result<Option<Field>> {
        match self.inner.next_field().await.map_err(Error::bad_request)? {
            Some(field) => Ok(Some(Field(field))),
            None => Ok(None),
        }
    }
}

pub struct Field(multer::Field<'static>);

impl Field {
    #[inline]
    pub fn content_type(&self) -> Option<&str> {
        self.0.content_type().map(|mime| mime.essence_str())
    }

    #[inline]
    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.0.name()
    }

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

    #[inline]
    pub async fn text(self) -> Result<String> {
        String::from_utf8(self.bytes().await?).map_err(Error::bad_request)
    }

    pub fn into_async_read(self) -> impl AsyncRead + Send {
        tokio_util::io::StreamReader::new(
            self.0
                .map_err(|err| std::io::Error::new(ErrorKind::Other, err.to_string())),
        )
    }
}
