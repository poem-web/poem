use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeekExt, ReadBuf, SeekFrom},
};

use crate::{error::ReadBodyError, FromRequest, Request, RequestBody};

/// An extractor that extracts the body and writes the contents to a temporary
/// file.
#[cfg_attr(docsrs, doc(cfg(feature = "tempfile")))]
pub struct TempFile(File);

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for TempFile {
    type Error = ReadBodyError;

    async fn from_request(_req: &'a Request, body: &mut RequestBody) -> Result<Self, Self::Error> {
        let body = body.take()?;
        let mut reader = body.into_async_read();
        let mut file = tokio::fs::File::from_std(::libtempfile::tempfile()?);
        tokio::io::copy(&mut reader, &mut file).await?;
        file.seek(SeekFrom::Start(0)).await?;
        Ok(Self(file))
    }
}

impl AsyncRead for TempFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;

    use super::*;
    use crate::{handler, Endpoint};

    #[tokio::test]
    async fn test_tempfile_extractor() {
        #[handler(internal)]
        async fn index123(mut file: TempFile) {
            let mut s = String::new();
            file.read_to_string(&mut s).await.unwrap();
            assert_eq!(s, "abcdef");
        }

        index123.call(Request::builder().body("abcdef")).await;
    }
}
