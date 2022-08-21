use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use futures_util::Stream;

use crate::{Metadata, Status, Streaming};

/// A GRPC response
pub struct Response<T> {
    pub(crate) metadata: Metadata,
    pub(crate) message: T,
}

impl<T: Debug> Debug for Response<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("metadata", &self.metadata)
            .field("message", &self.message)
            .finish()
    }
}

impl<T> Deref for Response<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl<T> DerefMut for Response<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
}

impl<T> Response<T> {
    /// Create a new `Response` with message `T`
    #[inline]
    pub fn new(message: T) -> Self {
        Self {
            metadata: Metadata::default(),
            message,
        }
    }

    /// Consumes this response object and returns inner message.
    #[inline]
    pub fn into_inner(self) -> T {
        self.message
    }

    /// Returns a reference to the metadata.
    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns a mutable reference to the metadata.
    #[inline]
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

impl<T> Response<Streaming<T>> {
    /// Create a new `Response` with `Streaming<T>`
    #[inline]
    pub fn new_streaming<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<T, Status>> + Send + 'static,
    {
        Self::new(Streaming::new(stream))
    }
}
