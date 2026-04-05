use crate::tokenizer::Span;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Ident<'src> {
    pub ident: &'src str,
    pub span: Span,
    pub renamings: usize,
}

impl<'src> Ident<'src> {
    pub fn name(&self) -> Name<'src> {
        Name::Ident(self.ident, self.renamings)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Name<'src> {
    Ident(&'src str, usize),
}

#[derive(Debug, Clone)]
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
    pub fn span(&self) -> Span {
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
            } => write!(f, "(λ{}.{})", parameter.ident, body),
            LambdaTerm::Application {
                function, argument, ..
            } => write!(f, "({} {})", function, argument),
        }
    }
}
