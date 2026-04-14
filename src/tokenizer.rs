#[derive(Debug, PartialEq, Eq)]
pub enum TokenKind<'src> {
    /// "λ"
    Lambda,
    /// "."
    Dot,
    /// "("
    LParen,
    /// ")"
    RParen,
    /// "let"
    Let,
    /// "="
    Equals,
    /// "in"
    In,
    Ident(&'src str),
    Eof,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token<'src> {
    pub kind: TokenKind<'src>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition(pub usize);

#[derive(Debug, PartialEq, Eq)]
pub struct Tokenizer<'src> {
    source: &'src [u8],
    position: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid token at position {}", position.0)]
pub struct InvalidTokenError {
    position: SourcePosition,
}

const LAMBDA_BYTES: [u8; 2] = const {
    let mut bytes: [u8; 2] = [0; 2];
    'λ'.encode_utf8(&mut bytes);
    bytes
};

impl<'src> Tokenizer<'src> {
    pub const fn new(source: &'src str) -> Self {
        Self {
            source: source.as_bytes(),
            position: 0,
        }
    }

    pub fn take_ident(&mut self) -> Result<Token<'src>, InvalidTokenError> {
        let Some(ident) = self.source[self.position..].utf8_chunks().next() else {
            return Err(InvalidTokenError {
                position: SourcePosition(self.position),
            });
        };

        let length: usize = ident
            .valid()
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
            .map(|ch| ch.len_utf8())
            .sum();

        if length == 0 {
            return Err(InvalidTokenError {
                position: SourcePosition(self.position),
            });
        }

        let ident = str::from_utf8(&self.source[self.position..self.position + length]).unwrap();
        let span = Span {
            start: SourcePosition(self.position),
            end: SourcePosition(self.position + length - 1),
        };

        self.position += length;

        let token = match ident {
            "let" => Token {
                kind: TokenKind::Let,
                span,
            },
            "in" => Token {
                kind: TokenKind::In,
                span,
            },
            _ => Token {
                kind: TokenKind::Ident(ident),
                span,
            },
        };

        Ok(token)
    }

    pub fn take_token(&mut self) -> Result<Token<'src>, InvalidTokenError> {
        let Some(byte) = self.source.get(self.position) else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span {
                    start: SourcePosition(self.position),
                    end: SourcePosition(self.position),
                },
            });
        };

        if byte == &LAMBDA_BYTES[0] {
            if self.source.get(self.position + 1) == Some(&LAMBDA_BYTES[1]) {
                let start = SourcePosition(self.position);
                let end = SourcePosition(self.position + 1);

                self.position += 2;

                return Ok(Token {
                    kind: TokenKind::Lambda,
                    span: Span { start, end },
                });
            }
        }

        let kind = match byte {
            b'.' => TokenKind::Dot,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'=' => TokenKind::Equals,
            b'/' if self.source.get(self.position + 1) == Some(&b'/') => {
                let length = self
                    .source
                    .get(self.position..)
                    .expect("at least comment char in comment")
                    .iter()
                    .position(|ch| *ch == b'\n')
                    .unwrap_or_else(|| self.source.len());
                self.position += length;
                return self.take_token();
            }
            byte if byte.is_ascii_whitespace() => {
                self.position += 1;
                return self.take_token();
            }
            _ => return self.take_ident(),
        };

        let position = SourcePosition(self.position);
        self.position += 1;
        return Ok(Token {
            kind,
            span: Span {
                start: position,
                end: position,
            },
        });
    }
}
