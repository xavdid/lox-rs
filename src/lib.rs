use std::{
    io::{self, Write},
    process::exit,
};

use crate::{evaluate::evaluate, parser::parse, reader::read_source, tokenize::tokenize};

pub mod config;
mod evaluate;
mod parser;
mod reader;
mod tokenize;

#[derive(Debug, PartialEq)]
pub enum LoxError {}

pub fn run_file(_file_path: &str) -> Result<(), LoxError> {
    // TODO: read file
    run("file contents")
}

// never returns; run until quit
pub fn run_repl() -> ! {
    // TODO: add environment
    loop {
        print!("lox.rs >>> ");
        io::stdout().flush().expect("flush to work");
        let mut buffer = String::new();
        let stdin = io::stdin(); // We get `Stdin` here.
        stdin
            .read_line(&mut buffer)
            .expect("reading from stdin to work");

        let input = buffer.trim();

        if input.is_empty() {
            println!("\nExiting!");
            exit(0);
        }

        // TODO: handle errors
        run(input).expect("no errors");
    }
}

// TODO: this should probably return something? for testability?
pub fn run(input: &str) -> Result<(), LoxError> {
    // this is the core of the interpreter
    let source = read_source(input);
    let tokens = tokenize(source);
    let ast = parse(tokens);
    evaluate(ast);

    Ok(())
}
