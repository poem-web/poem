use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{stream::BoxStream, Stream, StreamExt};

use crate::Status;

/// Message stream
pub struct Streaming<T>(BoxStream<'static, Result<T, Status>>);

impl<T> Streaming<T> {
    /// Create a message stream
    #[inline]
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<T, Status>> + Send + 'static,
    {
        Self(stream.boxed())
    }
}

impl<T> Stream for Streaming<T> {
    type Item = Result<T, Status>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}
