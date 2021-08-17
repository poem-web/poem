use tokio::{
    fs::File,
    io::{AsyncSeekExt, SeekFrom},
};

use crate::{error::ErrorBodyHasBeenTaken, Body, Error, FromRequest, Request};

/// An extractor that extracts the body and writes the contents to a temporary
/// file.
#[cfg_attr(docsrs, doc(cfg(feature = "tempfile")))]
pub struct TempFile(File);

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for TempFile {
    async fn from_request(_req: &'a Request, body: &mut Option<Body>) -> crate::Result<Self> {
        let body = body.take().ok_or(ErrorBodyHasBeenTaken)?;
        let mut reader = body.into_async_read();
        let mut file = tokio::fs::File::from_std(
            ::tempfile::tempfile().map_err(Error::internal_server_error)?,
        );
        tokio::io::copy(&mut reader, &mut file)
            .await
            .map_err(Error::internal_server_error)?;
        file.seek(SeekFrom::Start(0))
            .await
            .map_err(Error::internal_server_error)?;
        Ok(Self(file))
    }
}
