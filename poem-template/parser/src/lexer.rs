use std::borrow::Cow;

use crate::{
    input_source::{InputSource, Location},
    LexerError, Span,
};

mod char {
    #[inline]
    pub(super) fn is_ident_begin(ch: u8) -> bool {
        ch.is_ascii_alphabetic() || ch == b'_'
    }

    #[inline]
    pub(super) fn is_ident(ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || ch == b'_'
    }
}

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
    Ident(&'a str),
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

impl<'a> Token<'a> {
    #[inline]
    pub(crate) fn new(ty: TokenType<'a>, span: impl Into<Span>) -> Self {
        Self {
            ty,
            span: span.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Scope {
    Template,
    Variable,
    Tag,
}

pub(crate) struct Lexer<'a> {
    input: InputSource<'a>,
    scope: Scope,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input: InputSource::new(input.as_bytes()),
            scope: Scope::Template,
        }
    }

    fn parse_variable(&mut self) -> Result<Token<'a>, LexerError> {
        let start_loc = self.input.location();

        match self.input.next_char().expect("unexpected end") {
            b'|' => Ok(Token::new(TokenType::Pipe, self.input.span(start_loc))),
            b'(' => Ok(Token::new(TokenType::ParenOpen, self.input.span(start_loc))),
            b')' => Ok(Token::new(
                TokenType::ParenClose,
                self.input.span(start_loc),
            )),
            ch if char::is_ident_begin(ch) => self.parse_ident(start_loc),
            ch => Err(LexerError {
                span: Default::default(),
                message: format!("unexpected char: '{}'", ch as char).into(),
            }),
        }
    }

    #[inline]
    fn parse_ident(&mut self, start_loc: Location) -> Result<Token<'a>, LexerError> {
        self.input.skip_chars_if(char::is_ident);
        Ok(Token::new(
            TokenType::Ident(self.input.string(start_loc, self.input.location())),
            (start_loc, self.input.location()),
        ))
    }

    #[inline]
    fn parse_number(&mut self, start_loc: Location) -> Result<Token<'a>, LexerError> {}
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.input.is_empty() {
            return None;
        }

        match self.scope {
            Scope::Template => {
                let start_loc = self.input.location();

                if self.input.advance_if(b"{{") {
                    Some(Ok(Token::new(
                        TokenType::VariableStart,
                        (start_loc, self.input.location()),
                    )))
                } else if self.input.advance_if(b"{%") {
                    Some(Ok(Token::new(
                        TokenType::TagStart,
                        (start_loc, self.input.location()),
                    )))
                } else {
                    self.input.skip_raw_block();
                    Some(Ok(Token::new(
                        TokenType::Raw(self.input.string(start_loc, self.input.location())),
                        (start_loc, self.input.location()),
                    )))
                }
            }
            Scope::Variable => {
                self.input.skip_whitespace();
                if self.input.is_empty() {
                    return None;
                }
                Some(self.parse_variable())
            }
            Scope::Tag => todo!(),
        }
    }
}
