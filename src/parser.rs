use std::fmt::Display;

use crate::tokenizer::{self, Span, Token, TokenKind, Tokenizer};

#[derive(Debug)]
pub struct Ident<'src> {
    pub ident: &'src str,
    pub span: Span,
}

#[derive(Debug)]
pub enum LambdaTerm<'src> {
    Variable(Ident<'src>),
    Abstraction {
        parameter: Ident<'src>,
        body: Box<LambdaTerm<'src>>,
        head_span: Span,
        span: Span,
    },
    Application {
        function: Box<LambdaTerm<'src>>,
        argument: Box<LambdaTerm<'src>>,
        span: Span,
    },
}

impl<'src> LambdaTerm<'src> {
    fn span(&self) -> Span {
        match self {
            LambdaTerm::Variable(ident) => ident.span,
            LambdaTerm::Abstraction { span, .. } => *span,
            LambdaTerm::Application { span, .. } => *span,
        }
    }
}

impl<'src> Display for LambdaTerm<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LambdaTerm::Variable(var) => write!(f, "{}", var.ident),
            LambdaTerm::Abstraction {
                parameter, body, ..
            } => write!(f, "λ{}.{}", parameter.ident, body),
            LambdaTerm::Application {
                function, argument, ..
            } => write!(f, "({}){}", function, argument),
        }
    }
}

#[derive(Debug)]
enum Section<'src> {
    Eof,
    Paren { start: Token<'src> },
}

impl<'src> Section<'src> {
    fn closed_by_token(&self, token: &Token<'src>) -> bool {
        match self {
            Self::Eof => token.kind == TokenKind::Eof,
            Self::Paren { .. } => token.kind == TokenKind::RParen,
        }
    }
}

#[derive(Debug)]
pub struct Parser<'src> {
    tokenizer: Tokenizer<'src>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<'src> {
    #[error(transparent)]
    InvalidToken(#[from] tokenizer::InvalidTokenError),
    #[error("unexpected token: {0:?}")]
    UnexpectedToken(Token<'src>),
}

impl<'src> Parser<'src> {
    pub fn new(tokenizer: Tokenizer<'src>) -> Self {
        Self { tokenizer }
    }

    pub fn parse(&mut self) -> Result<LambdaTerm<'src>, Error<'src>> {
        self.parse_section(Section::Eof)
    }

    fn parse_section(&mut self, section: Section<'src>) -> Result<LambdaTerm<'src>, Error<'src>> {
        let mut terms = Vec::new();
        loop {
            let token = self.tokenizer.take_token()?;
            if section.closed_by_token(&token) {
                if terms.is_empty() {
                    return Err(Error::UnexpectedToken(token));
                }

                break;
            }

            let term = match token.kind {
                TokenKind::Ident(ident) => LambdaTerm::Variable(Ident {
                    ident: ident,
                    span: token.span,
                }),
                TokenKind::LParen => self.parse_section(Section::Paren { start: token })?,
                TokenKind::Lambda => {
                    terms.push(self.parse_abstraction(section, &token)?);
                    break;
                }
                TokenKind::Dot | TokenKind::Eof | TokenKind::RParen => {
                    return Err(Error::UnexpectedToken(token));
                }
            };

            terms.push(term)
        }

        Ok(terms
            .into_iter()
            .reduce(|lhs, rhs| LambdaTerm::Application {
                span: Span {
                    start: lhs.span().start,
                    end: lhs.span().end,
                },
                function: Box::new(lhs),
                argument: Box::new(rhs),
            })
            .expect("at least 1 term"))
    }

    fn parse_abstraction(
        &mut self,
        section: Section<'src>,
        lambda_token: &Token<'src>,
    ) -> Result<LambdaTerm<'src>, Error<'src>> {
        let parameter = self.tokenizer.take_token()?;
        let parameter = match &parameter.kind {
            TokenKind::Ident(ident) => Ident {
                ident,
                span: parameter.span,
            },
            _ => return Err(Error::UnexpectedToken(parameter)),
        };

        let dot = self.tokenizer.take_token()?;
        match dot.kind {
            TokenKind::Dot => {}
            _ => return Err(Error::UnexpectedToken(dot)),
        }

        let body = self.parse_section(section)?;
        let span = Span {
            start: lambda_token.span.start,
            end: body.span().end,
        };

        Ok(LambdaTerm::Abstraction {
            parameter,
            body: Box::new(body),
            head_span: Span {
                start: lambda_token.span.start,
                end: dot.span.end,
            },
            span,
        })
    }
}
