use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use futures_util::Stream;
use hyper::http::Extensions;

use crate::{Metadata, Status, Streaming};

/// A GRPC request
pub struct Request<T> {
    pub(crate) metadata: Metadata,
    pub(crate) message: T,
    pub(crate) extensions: Extensions,
}

impl<T: Debug> Debug for Request<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("metadata", &self.metadata)
            .field("message", &self.message)
            .finish()
    }
}

impl<T> Deref for Request<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl<T> DerefMut for Request<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
}

impl<T> Request<T> {
    /// Create a new `Request` with message `T`
    #[inline]
    pub fn new(message: T) -> Self {
        Self {
            metadata: Metadata::default(),
            message,
            extensions: Extensions::default(),
        }
    }

    /// Consumes this request object and returns inner message.
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

    /// Returns a reference to the associated extensions.
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Get a reference from extensions, similar to `self.extensions().get()`.
    #[inline]
    pub fn data<D: Send + Sync + 'static>(&self) -> Option<&D> {
        self.extensions.get()
    }

    /// Inserts a value to extensions, similar to
    /// `self.extensions().insert(data)`.
    #[inline]
    pub fn set_data(&mut self, data: impl Send + Sync + 'static) {
        self.extensions.insert(data);
    }
}

impl<T> Request<Streaming<T>> {
    /// Create a new `Request` with `Streaming<T>`
    #[inline]
    pub fn new_streaming<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<T, Status>> + Send + 'static,
    {
        Self::new(Streaming::new(stream))
    }
}
