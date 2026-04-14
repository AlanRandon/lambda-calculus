use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

mod ast;
mod parser;
mod reduce;
mod tokenizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = include_str!("input.lambda");
    let tokenizer = Tokenizer::new(src);
    let mut parser = Parser::new(tokenizer);
    let term = parser.parse()?;
    println!("parsed:\n{}", term.to_string());
    println!("reduced:\n{}", term.reduce_normal_par());
    Ok(())
}
