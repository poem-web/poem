use crate::{LineColumn, Span};

#[derive(Debug, Copy, Clone)]
pub(crate) struct Location {
    pos: LineColumn,
    idx: usize,
}

impl From<(Location, Location)> for Span {
    #[inline]
    fn from((start, end): (Location, Location)) -> Self {
        Self {
            start: start.pos,
            end: end.pos,
        }
    }
}

pub(crate) struct InputSource<'a> {
    pos: LineColumn,
    idx: usize,
    src: &'a [u8],
}

impl<'a> InputSource<'a> {
    #[inline]
    pub(crate) fn new(src: &'a [u8]) -> Self {
        Self {
            pos: LineColumn::new(1, 1),
            idx: 0,
            src,
        }
    }

    #[inline]
    pub(crate) fn location(&self) -> Location {
        Location {
            pos: self.pos,
            idx: self.idx,
        }
    }

    #[inline]
    pub(crate) fn span(&self, start: Location) -> Span {
        (start, self.location()).into()
    }

    #[inline]
    fn advance(&mut self, len: usize) {
        debug_assert!(self.idx + len <= self.src.len());

        for ch in &self.src[self.idx..self.idx + len] {
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

    #[inline]
    pub(crate) fn peek_char(&self) -> Option<u8> {
        if self.idx < self.src.len() {
            Some(self.src[self.idx])
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn next_char(&mut self) -> Option<u8> {
        match self.peek_char() {
            Some(ch) => {
                self.advance(1);
                Some(ch)
            }
            None => None,
        }
    }

    #[inline]
    pub(crate) fn skip_chars_if(&mut self, f: impl Fn(u8) -> bool) {
        loop {
            match self.peek_char() {
                Some(ch) if f(ch) => self.advance(1),
                _ => break,
            }
        }
    }

    pub(crate) fn advance_if(&mut self, needle: &[u8]) -> bool {
        if self.src.starts_with(needle) {
            self.advance(needle.len());
            true
        } else {
            false
        }
    }

    pub(crate) fn skip_raw_block(&mut self) {
        let idx = {
            let mut p = self.idx;

            loop {
                match memchr::memchr(b'{', &self.src[p..]) {
                    Some(idx)
                        if idx + 1 < self.src.len()
                            && (self.src[idx + 1] == b'{' || self.src[idx + 1] == b'%') =>
                    {
                        break p + idx;
                    }
                    Some(idx) => p += idx + 1,
                    None => break self.src.len(),
                };
            }
        };

        self.advance(idx - self.idx);
    }

    pub(crate) fn skip_whitespace(&mut self) {
        self.skip_chars_if(|ch| ch.is_ascii_whitespace());
    }

    #[inline]
    pub(crate) fn string(&self, start: Location, end: Location) -> &'a str {
        debug_assert!(end.idx >= start.idx);
        debug_assert!(start.idx <= self.src.len());
        debug_assert!(end.idx <= self.src.len());
        std::str::from_utf8(&self.src[start.idx..end.idx]).unwrap()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.idx == self.src.len()
    }
}
