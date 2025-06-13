use std::{io::Result as IoResult, str::FromStr};

use http::HeaderMap;

use crate::{Code, Metadata, Status};

/// The compression encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionEncoding {
    /// gzip
    #[cfg(feature = "gzip")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
    GZIP,
    /// deflate
    #[cfg(feature = "deflate")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deflate")))]
    DEFLATE,
    /// brotli
    #[cfg(feature = "brotli")]
    #[cfg_attr(docsrs, doc(cfg(feature = "brotli")))]
    BROTLI,
    /// zstd
    #[cfg(feature = "zstd")]
    #[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
    ZSTD,
}

impl FromStr for CompressionEncoding {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "gzip")]
            "gzip" => Ok(CompressionEncoding::GZIP),
            #[cfg(feature = "deflate")]
            "deflate" => Ok(CompressionEncoding::DEFLATE),
            #[cfg(feature = "brotli")]
            "br" => Ok(CompressionEncoding::BROTLI),
            #[cfg(feature = "zstd")]
            "zstd" => Ok(CompressionEncoding::ZSTD),
            _ => Err(()),
        }
    }
}

impl CompressionEncoding {
    /// Returns the encoding name.
    #[allow(unreachable_patterns)]
    pub fn as_str(&self) -> &'static str {
        match self {
            #[cfg(feature = "gzip")]
            CompressionEncoding::GZIP => "gzip",
            #[cfg(feature = "deflate")]
            CompressionEncoding::DEFLATE => "deflate",
            #[cfg(feature = "brotli")]
            CompressionEncoding::BROTLI => "br",
            #[cfg(feature = "zstd")]
            CompressionEncoding::ZSTD => "zstd",
            _ => unreachable!(),
        }
    }

    #[allow(
        unreachable_code,
        unused_imports,
        unused_mut,
        unused_variables,
        unreachable_patterns
    )]
    pub(crate) async fn encode(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let mut buf = Vec::new();

        match self {
            #[cfg(feature = "gzip")]
            CompressionEncoding::GZIP => {
                async_compression::tokio::bufread::GzipEncoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "deflate")]
            CompressionEncoding::DEFLATE => {
                async_compression::tokio::bufread::DeflateEncoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "brotli")]
            CompressionEncoding::BROTLI => {
                async_compression::tokio::bufread::BrotliEncoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "zstd")]
            CompressionEncoding::ZSTD => {
                async_compression::tokio::bufread::ZstdEncoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            _ => unreachable!(),
        }

        Ok(buf)
    }

    #[allow(
        unreachable_code,
        unused_imports,
        unused_mut,
        unused_variables,
        unreachable_patterns
    )]
    pub(crate) async fn decode(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let mut buf = Vec::new();

        match self {
            #[cfg(feature = "gzip")]
            CompressionEncoding::GZIP => {
                async_compression::tokio::bufread::GzipDecoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "deflate")]
            CompressionEncoding::DEFLATE => {
                async_compression::tokio::bufread::DeflateDecoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "brotli")]
            CompressionEncoding::BROTLI => {
                async_compression::tokio::bufread::BrotliDecoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            #[cfg(feature = "zstd")]
            CompressionEncoding::ZSTD => {
                async_compression::tokio::bufread::ZstdDecoder::new(data)
                    .read_to_end(&mut buf)
                    .await?;
            }
            _ => unreachable!(),
        }

        Ok(buf)
    }
}

fn unimplemented(accept_compressed: &[CompressionEncoding]) -> Status {
    let mut md = Metadata::new();
    let mut accept_encoding = String::new();
    let mut iter = accept_compressed.iter();
    if let Some(encoding) = iter.next() {
        accept_encoding.push_str(encoding.as_str());
    }
    for encoding in iter {
        accept_encoding.push_str(", ");
        accept_encoding.push_str(encoding.as_str());
    }
    md.append("grpc-accept-encoding", accept_encoding);
    Status::new(Code::Unimplemented)
        .with_metadata(md)
        .with_message("unsupported encoding")
}

#[allow(clippy::result_large_err)]
pub(crate) fn get_incoming_encodings(
    headers: &HeaderMap,
    accept_compressed: &[CompressionEncoding],
) -> Result<Option<CompressionEncoding>, Status> {
    let Some(value) = headers.get("grpc-encoding") else {
        return Ok(None);
    };
    let Some(encoding) = value
        .to_str()
        .ok()
        .and_then(|value| value.parse::<CompressionEncoding>().ok())
    else {
        return Err(unimplemented(accept_compressed));
    };
    if !accept_compressed.contains(&encoding) {
        return Err(unimplemented(accept_compressed));
    }
    Ok(Some(encoding))
}
