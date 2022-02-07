use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

#[derive(Debug, Copy, Clone, Default)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Copy, Clone, Default)]
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

#[derive(Debug, Copy, Clone)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
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

    pub fn wrap<R>(self, f: impl FnOnce(Spanned<T>) -> R) -> Spanned<R> {
        Spanned {
            span: self.span,
            value: f(self),
        }
    }

    pub fn spanned<R>(&self, value: R) -> Spanned<R> {
        Spanned {
            span: self.span,
            value,
        }
    }
}
