use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use std::borrow::Cow;
use std::collections::HashSet;

mod ast;
mod parser;
mod tokenizer;

impl<'src> ast::LambdaTerm<'src> {
    /// `FV(M)`
    fn free_variables(&self) -> HashSet<ast::Name<'src>> {
        match self {
            ast::LambdaTerm::Variable(ident) => HashSet::from([ident.name()]),
            ast::LambdaTerm::Abstraction {
                parameter, body, ..
            } => {
                let mut variables = body.free_variables();
                variables.remove(&parameter.name());
                variables
            }
            ast::LambdaTerm::Application {
                function, argument, ..
            } => function
                .free_variables()
                .union(&argument.free_variables())
                .cloned()
                .collect(),
        }
    }

    /// `M[x:=N]` substitution
    fn substitute(self, variable: ast::Name<'src>, term: Cow<Self>) -> Self {
        match self {
            // `x[x:=N]` -> `N`
            ast::LambdaTerm::Variable(ident) if ident.name() == variable => term.into_owned(),
            // `y[x:=N]` -> `y`
            ast::LambdaTerm::Variable(_) => self,
            // `(M_1 M_2)[x:=N]` -> `M_1[x:=N] M_2[x:=N]`
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => ast::LambdaTerm::Application {
                function: Box::new(function.substitute(variable.clone(), term.clone())),
                argument: Box::new(argument.substitute(variable, term)),
                span,
            },
            ast::LambdaTerm::Abstraction {
                parameter,
                body,
                head_span,
                span,
            } => {
                if parameter.name() == variable {
                    // `(λx.M)[x:=N]` -> `λx.M`
                    ast::LambdaTerm::Abstraction {
                        parameter,
                        head_span,
                        span,
                        body,
                    }
                } else {
                    let free_variables = term.free_variables();
                    if free_variables.contains(&parameter.name()) {
                        // `(λy.M)[x:=N]` -> `λy'.(M[y:=y'][x:=N])` if `y ∈ FV(N)`
                        let mut renamed_parameter = parameter.clone();
                        while free_variables.contains(&renamed_parameter.name()) {
                            renamed_parameter.renamings += 1;
                        }

                        let body = body
                            .substitute(
                                parameter.name(),
                                Cow::Owned(ast::LambdaTerm::Variable(renamed_parameter.clone())),
                            )
                            .substitute(variable, term);

                        ast::LambdaTerm::Abstraction {
                            parameter: renamed_parameter,
                            head_span,
                            span,
                            body: Box::new(body),
                        }
                    } else {
                        // `(λy.M)[x:=N]` -> `λy.(M[x:=N])` if `y ∉ FV(N)`
                        ast::LambdaTerm::Abstraction {
                            parameter,
                            head_span,
                            span,
                            body: Box::new(body.substitute(variable, term)),
                        }
                    }
                }
            }
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
                    .substitute(parameter.name(), Cow::Owned(*argument))
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
                    .substitute(parameter.name(), Cow::Owned(*argument))
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
    println!("parsed:\n{}", term.to_string());
    println!("reduced:\n{}", term.reduce_normal());
    Ok(())
}
