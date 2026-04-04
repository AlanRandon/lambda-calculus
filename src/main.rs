use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use std::borrow::Cow;

mod ast;
mod parser;
mod tokenizer;

impl<'src> ast::LambdaTerm<'src> {
    /// `M[N/x]` substitution
    fn substitute(self, variable: &str, term: Cow<Self>) -> Self {
        match self {
            ast::LambdaTerm::Variable(ident) if ident.ident == variable => term.into_owned(),
            ast::LambdaTerm::Variable(_) => self,
            ast::LambdaTerm::Abstraction { ref parameter, .. } if parameter.ident == variable => {
                self
            }
            ast::LambdaTerm::Abstraction {
                parameter,
                body,
                head_span,
                span,
            } => ast::LambdaTerm::Abstraction {
                parameter,
                head_span,
                span,
                body: Box::new(body.substitute(variable, term)),
            },
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => ast::LambdaTerm::Application {
                function: Box::new(function.substitute(variable, term.clone())),
                argument: Box::new(argument.substitute(variable, term)),
                span,
            },
        }
    }

    fn reduce_call_by_name(self) -> Self {
        match self {
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => match function.reduce_call_by_name() {
                ast::LambdaTerm::Abstraction {
                    parameter, body, ..
                } => body
                    .substitute(parameter.ident, Cow::Owned(*argument))
                    .reduce_call_by_name(),
                function => ast::LambdaTerm::Application {
                    function: Box::new(function),
                    argument,
                    span,
                },
            },
            term => term,
        }
    }

    fn reduce_normal(self) -> Self {
        match self {
            ast::LambdaTerm::Variable(_) => self,
            ast::LambdaTerm::Abstraction {
                parameter,
                body,
                head_span,
                span,
            } => ast::LambdaTerm::Abstraction {
                parameter,
                body: Box::new(body.reduce_normal()),
                head_span,
                span,
            },
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => match function.reduce_call_by_name() {
                ast::LambdaTerm::Abstraction {
                    parameter, body, ..
                } => body
                    .substitute(parameter.ident, Cow::Owned(*argument))
                    .reduce_normal(),
                function => ast::LambdaTerm::Application {
                    function: Box::new(function.reduce_normal()),
                    argument: Box::new(argument.reduce_normal()),
                    span,
                },
            },
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = include_str!("input.lambda");
    let tokenizer = Tokenizer::new(src);
    let mut parser = Parser::new(tokenizer);
    let term = parser.parse()?;
    println!("{} → {}", term.to_string(), term.reduce_normal());
    Ok(())
}
