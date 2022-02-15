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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

    fn advance(&mut self, len: usize) -> LineColumn {
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
        self.pos
    }

    fn parse_in_template(&mut self) -> Option<Result<Token<'a>, LexerError>> {
        let mut p = 0;
        let start_pos = self.pos;

        loop {
            match memchr::memchr(b'{', &self.src[p..]) {
                Some(idx) => {
                    if idx + 1 < self.src.len() {
                        if self.src[idx + 1] == b'{' {
                            let value = &self.src[p + idx];
                            let end_pos = self.advance(p + idx);
                            self.advance(2);
                            return Some(Ok(Token {
                                ty: TokenType::Raw(std::str::from_utf8(value).unwrap()),
                                span: Default::default(),
                            }));
                        } else if self.src[idx + 1] == b'%' {
                        }
                    }
                }
                None => {}
            }
        }

        // match self.src.get(..2) {
        //     Some(b"{{") => {
        //         self.advance(2);
        //         self.scope = Scope::Variable;
        //         Some(Ok(Token {
        //             ty: TokenType::VariableStart,
        //             span: Span::new(start_pos, self.pos),
        //         }))
        //     }
        //     Some(b"{%") => {
        //         self.advance(2);
        //         self.scope = Scope::Tag;
        //         Some(Ok(Token {
        //             ty: TokenType::TagStart,
        //             span: Span::new(start_pos, self.pos),
        //         }))
        //     }
        //     _ => {}
        // }
        todo!()
    }

    fn parse_in_variable(&mut self) -> Option<Result<Token<'a>, LexerError>> {
        todo!()
    }

    fn parse_in_tag(&mut self) -> Option<Result<Token<'a>, LexerError>> {
        todo!()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.src.is_empty() {
            return None;
        }

        match self.scope {
            Scope::Template => self.parse_in_template(),
            Scope::Variable => self.parse_in_variable(),
            Scope::Tag => self.parse_in_tag(),
        }
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
