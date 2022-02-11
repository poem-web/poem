use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

impl LineColumn {
    #[inline]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct Span {
    pub start: LineColumn,
    pub end: LineColumn,
}

impl Span {
    #[inline]
    pub fn new(start: LineColumn, end: LineColumn) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Spanned<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            span: Default::default(),
            value,
        }
    }

    pub fn map<R>(self, f: impl FnOnce(T) -> R) -> Spanned<R> {
        Spanned {
            span: self.span,
            value: f(self.value),
        }
    }
}
