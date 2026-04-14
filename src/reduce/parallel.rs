use crate::ast;
use std::borrow::Cow;
use std::collections::HashSet;

impl<'src> ast::LambdaTerm<'src> {
    /// `FV(M)`
    pub fn free_variables_par(&self) -> HashSet<ast::Name<'src>> {
        match self {
            ast::LambdaTerm::Variable(ident) => HashSet::from([ident.name()]),
            ast::LambdaTerm::Abstraction {
                parameter, body, ..
            } => {
                let mut variables = body.free_variables_par();
                variables.remove(&parameter.name());
                variables
            }
            ast::LambdaTerm::Application {
                function, argument, ..
            } => {
                let (function_set, argument_set) = rayon::join(
                    || function.free_variables_par(),
                    || argument.free_variables_par(),
                );

                function_set.union(&argument_set).cloned().collect()
            }
        }
    }

    /// `M[x:=N]` substitution
    pub fn substitute_par(self, variable: ast::Name<'src>, term: Cow<Self>) -> Self {
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
            } => {
                let (function, argument) = rayon::join(
                    {
                        let variable = variable.clone();
                        let term = term.clone();
                        || Box::new(function.substitute_par(variable, term))
                    },
                    || Box::new(argument.substitute_par(variable, term)),
                );

                ast::LambdaTerm::Application {
                    function,
                    argument,
                    span,
                }
            }
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
                    let free_variables = term.free_variables_par();
                    if free_variables.contains(&parameter.name()) {
                        // `(λy.M)[x:=N]` -> `λy'.(M[y:=y'][x:=N])` if `y ∈ FV(N)`
                        let mut renamed_parameter = parameter.clone();
                        while free_variables.contains(&renamed_parameter.name()) {
                            renamed_parameter.renamings += 1;
                        }

                        let body = body
                            .substitute_par(
                                parameter.name(),
                                Cow::Owned(ast::LambdaTerm::Variable(renamed_parameter.clone())),
                            )
                            .substitute_par(variable, term);

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
                            body: Box::new(body.substitute_par(variable, term)),
                        }
                    }
                }
            }
        }
    }

    pub fn reduce_call_by_name_par(self) -> Self {
        match self {
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => match function.reduce_call_by_name_par() {
                ast::LambdaTerm::Abstraction {
                    parameter, body, ..
                } => body
                    .substitute_par(parameter.name(), Cow::Owned(*argument))
                    .reduce_call_by_name_par(),
                function => ast::LambdaTerm::Application {
                    function: Box::new(function),
                    argument,
                    span,
                },
            },
            term => term,
        }
    }

    pub fn reduce_normal_par(self) -> Self {
        match self {
            ast::LambdaTerm::Variable(_) => self,
            ast::LambdaTerm::Abstraction {
                parameter,
                body,
                head_span,
                span,
            } => ast::LambdaTerm::Abstraction {
                parameter,
                body: Box::new(body.reduce_normal_par()),
                head_span,
                span,
            },
            ast::LambdaTerm::Application {
                function,
                argument,
                span,
            } => match function.reduce_call_by_name_par() {
                ast::LambdaTerm::Abstraction {
                    parameter, body, ..
                } => body
                    .substitute_par(parameter.name(), Cow::Owned(*argument))
                    .reduce_normal_par(),
                function => {
                    let (function, argument) = rayon::join(
                        || Box::new(function.reduce_normal_par()),
                        || Box::new(argument.reduce_normal_par()),
                    );

                    ast::LambdaTerm::Application {
                        function,
                        argument,
                        span,
                    }
                }
            },
        }
    }
}
