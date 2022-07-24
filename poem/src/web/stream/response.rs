use bytes::Bytes;
use futures_util::{stream::BoxStream, Stream, StreamExt};

use crate::{Body, IntoResponse, Response};

/// A Bytes streaming response.
///
/// # Example
///
/// ```
/// use bytes::Bytes;
/// use futures_util::stream;
/// use poem::{
///     handler,
///     test::TestClient,
///     web::stream::StreamResponse,
/// };
///
/// #[handler]
/// fn index() -> StreamResponse {
///     StreamResponse::new(stream::iter(vec![
///         Bytes::from("abc"),
///         Bytes::from("def"),
///         Bytes::from("ghi"),
///     ]))
/// }
///
/// let cli = TestClient::new(index);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("abcdefghi").await;
/// # });
/// ```
pub struct StreamResponse {
    stream: BoxStream<'static, Bytes>,
}

impl StreamResponse {
    /// Creates a response from a stream of Bytes.
    pub fn new(stream: impl Stream<Item = Bytes> + Send + 'static) -> Self {
        Self {
            stream: stream.boxed(),
        }
    }
}

impl IntoResponse for StreamResponse {
    fn into_response(self) -> Response {
        let stream = self
            .stream
            .map(|chunk| Ok::<_, std::io::Error>(chunk))
            .boxed();

        Response::builder()
            .content_type("application/octet-stream")
            .body(Body::from_async_read(tokio_util::io::StreamReader::new(
                stream,
            )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create() {
        let st = futures_util::stream::iter([Bytes::from("abc"), Bytes::from("def")].into_iter());
        let res = StreamResponse::new(st).into_response();
        assert_eq!(res.content_type().unwrap(), "application/octet-stream");
        assert_eq!(res.into_body().into_string().await.unwrap(), "abcdef");
    }

    #[tokio::test]
    async fn test_custom_content_type() {
        let st = futures_util::stream::iter(
            [Bytes::from(r#""abc"#), Bytes::from(r#"def""#)].into_iter(),
        );
        let res = StreamResponse::new(st)
            .with_content_type("application/json")
            .into_response();
        assert_eq!(res.content_type().unwrap(), "application/json");
        assert_eq!(
            res.into_body().into_json::<String>().await.unwrap(),
            "abcdef"
        );
    }
}
