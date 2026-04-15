use crate::parser::Parser;
use crate::tokenizer::Tokenizer;
use clap::Parser as _;
use miette::{Diagnostic, IntoDiagnostic, NamedSource, Report, SourceSpan};
use std::borrow::Cow;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;

mod ast;
mod parser;
mod reduce;
mod tokenizer;

#[derive(clap::Parser)]
struct Cli {
    #[arg(default_value = "src/input.lambda")]
    input: PathBuf,
    #[arg(long = "reduce", short = 'r', default_value = "normal-parallel")]
    reduction_stategy: ReductionStrategy,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum ReductionStrategy {
    Normal,
    NormalParallel,
    CallByName,
    CallByNameParallel,
}

#[derive(Error, Debug, Diagnostic)]
enum ParseDiagnosticKind {
    #[error("unexpected token")]
    Parse {
        #[label]
        span: SourceSpan,
    },
    #[error("unexpected EOF")]
    Eof,
    #[error("invalid token")]
    Tokenize {
        #[label]
        span: SourceSpan,
    },
    #[error("error parsing `{section}`")]
    Section {
        section: &'static str,
        #[label = "`{section}` starts here"]
        span: SourceSpan,
        #[source]
        #[diagnostic_source]
        diagnostic: Box<dyn Diagnostic + Send + Sync>,
    },
}

impl From<tokenizer::Span> for SourceSpan {
    fn from(span: tokenizer::Span) -> Self {
        SourceSpan::new(span.start.0.into(), span.end.0 - span.start.0 + 1)
    }
}

impl From<tokenizer::SourcePosition> for SourceSpan {
    fn from(position: tokenizer::SourcePosition) -> Self {
        tokenizer::Span {
            start: position,
            end: position,
        }
        .into()
    }
}

impl<'src> From<parser::Error<'src>> for ParseDiagnosticKind {
    fn from(err: parser::Error<'src>) -> Self {
        match err {
            parser::Error::InvalidToken(err) => ParseDiagnosticKind::Tokenize {
                span: SourceSpan::new(err.position.0.into(), 0),
            },
            parser::Error::UnexpectedToken(token) => {
                let span = token.span.into();
                match token.kind {
                    tokenizer::TokenKind::Eof => ParseDiagnosticKind::Eof,
                    _ => ParseDiagnosticKind::Parse { span },
                }
            }
            parser::Error::Section { open_token, error } => ParseDiagnosticKind::Section {
                section: match open_token.kind {
                    tokenizer::TokenKind::LParen => "( … )",
                    tokenizer::TokenKind::Let => "let … = … in …",
                    _ => unimplemented!(),
                },
                span: open_token.span.into(),
                diagnostic: Box::new(ParseDiagnosticKind::from(*error)),
            },
        }
    }
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let mut src = String::new();
    let name = match cli.input.as_os_str().as_encoded_bytes() {
        b"-" => {
            std::io::stdin()
                .read_to_string(&mut src)
                .into_diagnostic()?;
            Cow::Borrowed("STDIN")
        }
        _ => {
            src = std::fs::read_to_string(&cli.input).into_diagnostic()?;
            cli.input.to_string_lossy()
        }
    };

    let tokenizer = Tokenizer::new(&src);
    let mut parser = Parser::new(tokenizer);
    match parser.parse() {
        Ok(term) => {
            println!("parsed:\n{}", term.to_string());

            let reduced = match cli.reduction_stategy {
                ReductionStrategy::Normal => term.reduce_normal(),
                ReductionStrategy::NormalParallel => term.reduce_normal_par(),
                ReductionStrategy::CallByName => term.reduce_call_by_name(),
                ReductionStrategy::CallByNameParallel => term.reduce_call_by_name_par(),
            };

            println!("reduced:\n{reduced}");
        }
        Err(err) => {
            let err = ParseDiagnosticKind::from(err);
            let err = Report::from(err);
            return Err(err.with_source_code(NamedSource::new(name, src)));
        }
    }

    Ok(())
}
