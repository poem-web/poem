use std::borrow::Cow;

use crate::{LexerError, LineColumn, Span};

#[derive(Debug, PartialEq)]
pub(crate) enum TokenType<'a> {
    /// Raw template data
    Raw(&'a str),
    /// Variable start
    VariableStart,
    /// Variable end
    VariableEnd,
    /// Tag start
    TagStart,
    /// Tag end
    TagEnd,
    /// An identifier
    Ident,
    /// An integer
    Int(i64),
    /// A float
    Float(f64),
    /// A string
    String(Cow<'a, str>),
    /// `+` operator
    Plus,
    /// `-` operator
    Minus,
    /// `*` operator
    Mul,
    /// `/` operator
    Div,
    /// `%` operator
    Rem,
    /// `=` operator
    Assign,
    /// `==` operator
    Eq,
    /// `!=` operator
    Ne,
    /// `<=` operator
    Le,
    /// `<` operator
    Lt,
    /// `>=` operator
    Ge,
    /// `>` operator
    Gt,
    /// `|` operator
    Pipe,
    /// `.` operator
    Dot,
    /// Open parenthesis
    ParenOpen,
    /// Close parenthesis
    ParenClose,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Token<'a> {
    pub(crate) ty: TokenType<'a>,
    pub(crate) span: Span,
}

#[derive(Debug)]
enum Scope {
    Template,
    Variable,
    Tag,
}

pub(crate) struct Lexer<'a> {
    pos: LineColumn,
    src: &'a [u8],
    scope: Scope,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            pos: LineColumn { line: 1, column: 1 },
            src: input.as_bytes(),
            scope: Scope::Template,
        }
    }

    fn advance(&mut self, len: usize) {
        for ch in &self.src[..len] {
            match *ch {
                b'\n' => {
                    self.pos.line += 1;
                    self.pos.column = 1;
                }
                _ => self.pos.column += 1,
            }
        }
        self.src = &self.src[len..];
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.src.is_empty() {
            return None;
        }

        let len = self.src.len();
        Some(match self.src[0] {
            b'{' if len > 1 && self.src[1] == b'{' => self.parse_variable(),
            b'{' if len > 1 && self.src[1] == b'%' => self.parse_tag(),
            _ => self.parse_raw(),
        })
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     fn check_tokens(input: &[u8], tokens: Vec<Result<TemplateToken,
// LexerError>>) {         let lexer = TemplateLexer::new(input);
//         assert_eq!(lexer.collect::<Vec<_>>(), tokens);
//     }
//
//     #[test]
//     fn test_variable() {
//         check_tokens(
//             b"{{ abc }}",
//             vec![Ok(TemplateToken {
//                 ty: TemplateTokenType::Variable,
//                 span: Span {
//                     start: LineColumn::new(1, 3),
//                     end: LineColumn::new(1, 8),
//                 },
//                 value: b" abc ",
//             })],
//         );
//
//         check_tokens(
//             b"{{ abc }} def {{ ghi }}",
//             vec![
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Variable,
//                     span: Span {
//                         start: LineColumn::new(1, 3),
//                         end: LineColumn::new(1, 8),
//                     },
//                     value: b" abc ",
//                 }),
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Raw,
//                     span: Span {
//                         start: LineColumn::new(1, 10),
//                         end: LineColumn::new(1, 15),
//                     },
//                     value: b" def ",
//                 }),
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Variable,
//                     span: Span {
//                         start: LineColumn::new(1, 17),
//                         end: LineColumn::new(1, 22),
//                     },
//                     value: b" ghi ",
//                 }),
//             ],
//         );
//     }
//
//     #[test]
//     fn test_tag() {
//         check_tokens(
//             b"{% abc %}",
//             vec![Ok(TemplateToken {
//                 ty: TemplateTokenType::Tag,
//                 span: Span {
//                     start: LineColumn::new(1, 3),
//                     end: LineColumn::new(1, 8),
//                 },
//                 value: b" abc ",
//             })],
//         );
//
//         check_tokens(
//             b"{% abc %} def {% ghi %}",
//             vec![
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Tag,
//                     span: Span {
//                         start: LineColumn::new(1, 3),
//                         end: LineColumn::new(1, 8),
//                     },
//                     value: b" abc ",
//                 }),
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Raw,
//                     span: Span {
//                         start: LineColumn::new(1, 10),
//                         end: LineColumn::new(1, 15),
//                     },
//                     value: b" def ",
//                 }),
//                 Ok(TemplateToken {
//                     ty: TemplateTokenType::Tag,
//                     span: Span {
//                         start: LineColumn::new(1, 17),
//                         end: LineColumn::new(1, 22),
//                     },
//                     value: b" ghi ",
//                 }),
//             ],
//         );
//     }
//
//     #[test]
//     fn test_unterminated_variable() {
//         let mut lexer = TemplateLexer::new(b"abc {{ abc");
//
//         assert_eq!(
//             lexer.next(),
//             Some(Ok(TemplateToken {
//                 ty: TemplateTokenType::Raw,
//                 span: Span::new(LineColumn::new(1, 1), LineColumn::new(1,
// 5)),                 value: b"abc "
//             }))
//         );
//
//         assert_eq!(
//             lexer.next(),
//             Some(Err(LexerError {
//                 span: Span::new(LineColumn::new(1, 7), LineColumn::new(1,
// 11)),                 message: "unterminated variable"
//             }))
//         );
//     }
//
//     #[test]
//     fn test_unterminated_tag() {
//         let mut lexer = TemplateLexer::new(b"abc {% abc");
//
//         assert_eq!(
//             lexer.next(),
//             Some(Ok(TemplateToken {
//                 ty: TemplateTokenType::Raw,
//                 span: Span::new(LineColumn::new(1, 1), LineColumn::new(1,
// 5)),                 value: b"abc "
//             }))
//         );
//
//         assert_eq!(
//             lexer.next(),
//             Some(Err(LexerError {
//                 span: Span::new(LineColumn::new(1, 7), LineColumn::new(1,
// 11)),                 message: "unterminated tag"
//             }))
//         );
//     }
// }
