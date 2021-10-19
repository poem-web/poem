//! Server-Sent Events (SSE) types.

mod event;
mod response;

pub use event::Event;
pub use response::SSE;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{io::AsyncReadExt, time::Instant};

    use super::*;
    use crate::IntoResponse;

    #[tokio::test]
    async fn sse() {
        let sse = SSE::new(futures_util::stream::iter(
            vec![1, 2, 3, 4, 5].into_iter().map(|value| {
                if value == 3 {
                    Event::message(value.to_string()).id("a").event_type("tt")
                } else if value == 4 {
                    Event::message(value.to_string())
                        .id("b")
                        .event_type("message")
                } else {
                    Event::message(value.to_string())
                }
            }),
        ));
        let resp = sse.into_response();

        assert_eq!(resp.content_type(), Some("text/event-stream"));
        let data = resp.into_body().into_string().await.unwrap();
        assert_eq!(
            data,
            r#"data: 1

data: 2

id: a
event: tt
data: 3

id: b
data: 4

data: 5

"#
        );
    }

    #[tokio::test]
    async fn keep_alive() {
        let sse = SSE::new(futures_util::stream::pending()).keep_alive(Duration::from_secs(1));
        let resp = sse.into_response();
        let mut body = resp.into_body().into_async_read();
        let mut s = Instant::now();

        for _ in 0..3 {
            let mut buf = [0; 16];
            assert_eq!(body.read(&mut buf).await.unwrap(), 3);
            assert_eq!(&buf[..3], b":\n\n");

            let now = Instant::now();
            let interval = now - s;
            assert!(interval >= Duration::from_millis(900) && interval < Duration::from_secs(2));
            s = now;
        }
    }
}
