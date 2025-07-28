use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use http_body::{Body, Frame, SizeHint};

pub struct WorkerBody(pub(crate) worker::Body);

impl Body for WorkerBody {
    type Data = bytes::Bytes;
    type Error = io::Error;

    #[inline]
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let body = self.get_mut();

        let inner = Pin::new(&mut body.0);

        let res = inner.poll_frame(cx);

        match res {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(r))) => Poll::Ready(Some(Ok(r))),
            Poll::Ready(Some(Err(e))) => match e {
                worker::Error::Io(e) => Poll::Ready(Some(Err(io::Error::other(e)))),
                _ => Poll::Ready(Some(Err(io::Error::other(e)))),
            },
        }
    }

    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
}

pub fn build_worker_body(body: poem::Body) -> Result<worker::Body, worker::Error> {
    let stream = body.into_bytes_stream();

    worker::Body::from_stream(stream)
}
