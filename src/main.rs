use crate::{parser::Parser, tokenizer::Tokenizer};

mod parser;
mod tokenizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = include_str!("input.lambda");
    let tokenizer = Tokenizer::new(src);
    let mut parser = Parser::new(tokenizer);
    let term = parser.parse()?;
    println!("{term:?}");
    println!("{term}");
    Ok(())
}
