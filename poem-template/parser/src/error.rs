use crate::Span;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error("[{}:{}-{}:{}] {message}", .span.start.line, .span.start.column, .span.end.line, .span.end.column)]
pub struct LexerError {
    pub span: Span,
    pub message: &'static str,
}
