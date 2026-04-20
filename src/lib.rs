use std::{
    io::{self, Write},
    process::exit,
};

use crate::{
    evaluate::evaluate,
    parser::parse,
    reader::{Source, read_source},
    tokenize::tokenize,
};

pub mod config;
mod evaluate;
mod parser;
mod reader;
mod tokenize;

#[derive(Debug)]
pub enum LoxError {
    ReaderError(io::Error),
}

impl From<io::Error> for LoxError {
    fn from(value: io::Error) -> Self {
        LoxError::ReaderError(value)
    }
}

pub fn run_file(file_path: &str) -> Result<(), LoxError> {
    // TODO: read file
    let source = read_source(file_path)?;
    println!("{}", source.text);
    run(&source)
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
        run(&Source {
            text: input.to_string(),
        })
        .expect("no errors");
    }
}

// TODO: this should probably return something? for testability?
pub fn run(input: &Source) -> Result<(), LoxError> {
    // this is the core of the interpreter
    let tokens = tokenize(input);
    let ast = parse(tokens);
    evaluate(ast);

    Ok(())
}
