use crate::ast;
use crate::tokenizer::{self, Span, Token, TokenKind, Tokenizer};

#[derive(Debug, Clone)]
pub enum Section {
    Eof,
    Paren,
    LetDeclaration,
}

impl<'src> Section {
    fn closed_by_token(&self, token: &Token<'src>) -> bool {
        match self {
            Self::Eof => token.kind == TokenKind::Eof,
            Self::Paren => token.kind == TokenKind::RParen,
            Self::LetDeclaration => token.kind == TokenKind::In,
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
    #[error("error parsing section")]
    Section {
        open_token: Token<'src>,
        error: Box<Error<'src>>,
    },
}

impl<'src> Parser<'src> {
    pub fn new(tokenizer: Tokenizer<'src>) -> Self {
        Self { tokenizer }
    }

    pub fn parse(&mut self) -> Result<ast::LambdaTerm<'src>, Error<'src>> {
        self.parse_section_inner(Section::Eof)
    }

    fn parse_section(
        &mut self,
        section: Section,
        open_token: Token<'src>,
    ) -> Result<ast::LambdaTerm<'src>, Error<'src>> {
        self.parse_section_inner(section.clone())
            .map_err(|err| Error::Section {
                open_token,
                error: Box::new(err),
            })
    }

    fn parse_section_inner(
        &mut self,
        section: Section,
    ) -> Result<ast::LambdaTerm<'src>, Error<'src>> {
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
                TokenKind::Ident(ident) => ast::LambdaTerm::Variable(ast::Ident {
                    ident: ident,
                    span: token.span,
                    renamings: 0,
                }),
                TokenKind::LParen => self.parse_section(Section::Paren, token)?,
                TokenKind::Lambda => {
                    terms.push(self.parse_abstraction(section, &token)?);
                    break;
                }
                TokenKind::Dot
                | TokenKind::Eof
                | TokenKind::RParen
                | TokenKind::Equals
                | TokenKind::In => {
                    return Err(Error::UnexpectedToken(token));
                }
                TokenKind::Let => {
                    terms.push(self.parse_let(section, token)?);
                    break;
                }
            };

            terms.push(term)
        }

        Ok(terms
            .into_iter()
            .reduce(|lhs, rhs| ast::LambdaTerm::Application {
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
        section: Section,
        lambda_token: &Token<'src>,
    ) -> Result<ast::LambdaTerm<'src>, Error<'src>> {
        let parameter = self.tokenizer.take_token()?;
        let parameter = match &parameter.kind {
            TokenKind::Ident(ident) => ast::Ident {
                ident,
                span: parameter.span,
                renamings: 0,
            },
            _ => return Err(Error::UnexpectedToken(parameter)),
        };

        let dot = self.tokenizer.take_token()?;
        match dot.kind {
            TokenKind::Dot => {}
            _ => return Err(Error::UnexpectedToken(dot)),
        }

        let body = self.parse_section_inner(section)?;
        let span = Span {
            start: lambda_token.span.start,
            end: body.span().end,
        };

        Ok(ast::LambdaTerm::Abstraction {
            parameter,
            body: Box::new(body),
            head_span: Span {
                start: lambda_token.span.start,
                end: dot.span.end,
            },
            span,
        })
    }

    fn parse_let(
        &mut self,
        section: Section,
        let_token: Token<'src>,
    ) -> Result<ast::LambdaTerm<'src>, Error<'src>> {
        let binding = self.tokenizer.take_token()?;
        let binding = match &binding.kind {
            TokenKind::Ident(ident) => ast::Ident {
                ident,
                span: binding.span,
                renamings: 0,
            },
            _ => return Err(Error::UnexpectedToken(binding)),
        };

        let equals = self.tokenizer.take_token()?;
        match equals.kind {
            TokenKind::Equals => {}
            _ => return Err(Error::UnexpectedToken(equals)),
        }

        let value = self.parse_section(Section::LetDeclaration, let_token.clone())?;
        let body = self.parse_section_inner(section)?;

        // `let x = y in z` -> `(λx.z)y`

        Ok(ast::LambdaTerm::Application {
            span: Span {
                start: let_token.span.start,
                end: body.span().end,
            },
            function: Box::new(ast::LambdaTerm::Abstraction {
                parameter: binding,
                span: Span {
                    start: let_token.span.start,
                    end: body.span().end,
                },
                body: Box::new(body),
                head_span: Span {
                    start: let_token.span.start,
                    end: value.span().end,
                },
            }),
            argument: Box::new(value),
        })
    }
}
