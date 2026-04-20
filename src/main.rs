use crate::{evaluate::evaluate, parser::parse, reader::read_source, tokenize::tokenize};

mod evaluate;
mod parser;
mod reader;
mod tokenize;

fn main() {
    let source = read_source();
    let tokens = tokenize(source);
    let ast = parse(tokens);
    evaluate(ast);
}
