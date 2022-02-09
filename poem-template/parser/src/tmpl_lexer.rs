use crate::{LexerError, LineColumn, Span};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TemplateTokenType {
    Raw,
    Variable,
    Tag,
}

pub(crate) struct TemplateToken<'a> {
    ty: TemplateTokenType,
    span: Span,
    value: &'a [u8],
}

pub(crate) struct TemplateLexer<'a> {
    pos: LineColumn,
    src: &'a [u8],
}

impl<'a> TemplateLexer<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            pos: LineColumn { line: 1, column: 1 },
            src: input,
        }
    }

    fn advance(&mut self, len: usize) {
        for ch in &self.src[len..] {
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

    fn parse_variable(&mut self) -> Result<TemplateToken<'a>, LexerError> {
        let start_pos = self.pos;
        self.advance(2);

        match memchr::memmem::find(self.src, b"}}") {
            Some(idx) => {
                let value = &self.src[..idx];
                self.advance(idx);
                Ok(TemplateToken {
                    ty: TemplateTokenType::Variable,
                    span: Span::new(start_pos, self.pos),
                    value,
                })
            }
            None => {
                self.advance(self.src.len());
                Err(LexerError {
                    span: Span::new(start_pos, self.pos),
                    message: "unterminated variable",
                })
            }
        }
    }

    fn parse_tag(&mut self) -> Result<TemplateToken<'a>, LexerError> {
        let start_pos = self.pos;
        self.advance(2);

        match memchr::memmem::find(self.src, b"%}") {
            Some(idx) => {
                let value = &self.src[..idx];
                self.advance(idx);
                Ok(TemplateToken {
                    ty: TemplateTokenType::Tag,
                    span: Span::new(start_pos, self.pos),
                    value,
                })
            }
            None => {
                self.advance(self.src.len());
                Err(LexerError {
                    span: Span::new(start_pos, self.pos),
                    message: "unterminated tag",
                })
            }
        }
    }

    fn parse_raw(&mut self) -> Result<TemplateToken<'a>, LexerError> {
        let mut p = 0;
        memchr::memchr2(b'}', b'%', self.src);
        todo!()
    }
}

impl<'a> Iterator for TemplateLexer<'a> {
    type Item = Result<TemplateToken<'a>, LexerError>;

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
