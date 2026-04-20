use crate::{evaluate::evaluate, parser::parse, reader::read_source, tokenize::tokenize};

mod evaluate;
mod parser;
mod reader;
mod tokenize;

fn main() {
    read_source();
    tokenize();
    parse();
    evaluate();
}
