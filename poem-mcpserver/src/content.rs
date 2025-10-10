//! Content types.

use std::fmt::Display;

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::protocol::content::Content;

/// Represents a type that can be converted into a content.
pub trait IntoContent {
    /// Consumes the object and converts it into a content.
    fn into_content(self) -> Content;
}

impl IntoContent for Content {
    #[inline]
    fn into_content(self) -> Content {
        self
    }
}

/// Represents a type that can be converted into multiple contents.
pub trait IntoContents {
    /// Consumes the object and converts it into multiple contents.
    fn into_contents(self) -> Vec<Content>;
}

impl<T> IntoContents for T
where
    T: IntoContent,
{
    fn into_contents(self) -> Vec<Content> {
        vec![self.into_content()]
    }
}

impl<T> IntoContents for Vec<T>
where
    T: IntoContent,
{
    fn into_contents(self) -> Vec<Content> {
        self.into_iter().map(IntoContent::into_content).collect()
    }
}

/// A wrapper type for multiple contents from an iterator.
pub struct ContentsIter<T>(pub T);

impl<T> IntoContents for ContentsIter<T>
where
    T: IntoIterator,
    T::Item: IntoContent,
{
    fn into_contents(self) -> Vec<Content> {
        self.0.into_iter().map(IntoContent::into_content).collect()
    }
}

/// A text response.
#[derive(Debug)]
pub struct Text<T>(pub T);

impl<T> IntoContent for Text<T>
where
    T: Display,
{
    fn into_content(self) -> Content {
        Content::Text {
            text: self.0.to_string(),
        }
    }
}

/// An image response.
#[derive(Debug)]
pub struct Image<T> {
    data: T,
    mime_type: String,
}

impl<T> Image<T> {
    /// Creates a image content.
    #[inline]
    pub fn new(data: T, mime_type: impl Into<String>) -> Self {
        Self {
            data,
            mime_type: mime_type.into(),
        }
    }
}

impl<T> IntoContent for Image<T>
where
    T: AsRef<[u8]>,
{
    fn into_content(self) -> Content {
        Content::Image {
            data: STANDARD.encode(self.data),
            mime_type: self.mime_type,
        }
    }
}
