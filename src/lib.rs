use std::{
    fmt::Display,
    io::{self, Write},
    process::exit,
};

use anyhow::{Error, Result, anyhow};

use crate::{
    interpreter::interpret,
    parser::parse,
    reader::{Source, read_source},
    scanner::tokenize,
};

mod ast;
pub mod config;
mod environment;
mod interpreter;
mod parser;
mod reader;
mod scanner;

#[derive(Debug)]
pub enum LoxError {
    Reader(String),
    Scanner(String),
    Parser(String),
    Interpreter(String),
}

impl Display for LoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            LoxError::Reader(m) => m,
            LoxError::Scanner(m) => m,
            LoxError::Parser(m) => m,
            LoxError::Interpreter(m) => m,
        };
        write!(f, "{message}")
    }
}

impl From<LoxError> for anyhow::Error {
    fn from(value: LoxError) -> Self {
        anyhow!(value)
    }
}

pub fn run_file(file_path: &str) -> Result<()> {
    let source = read_source(file_path)?;
    run(&source)
}

/**
 * Given a bunch of Errors, turn them into one newline-separated list.
 *
 * Bridges the gap between things like the parser (which can return many errors) and main, which expects a single one.
 */
fn join_errors(errors: Vec<Error>) -> Error {
    let joined: String = errors.iter().map(|e| format!("{e}\n")).collect();
    anyhow!(joined)
}

// never returns; run until quit
pub fn run_repl() -> ! {
    // TODO: add environment
    // TODO: print expressions, e.g.
    // >>> 1 + 1;
    // > 2
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

        if let Err(e) = run(&Source {
            text: input.to_string(),
        }) {
            eprintln!("{e}")
        }
    }
}

pub fn run(input: &Source) -> Result<()> {
    // this is the core of the interpreter
    let tokens = tokenize(input)?;
    let ast = parse(tokens)?;
    interpret(ast).unwrap(); // TODO: Fix

    Ok(())
}

#[cfg(test)]
pub mod test_util {
    use anyhow::Error;

    /** test helper for making assertions about error messages. Performs a case-insensitive match on the error message  */
    pub fn assert_contains(err: &Error, msg: &str) {
        let input = err.to_string().to_lowercase();
        let substr = &msg.to_lowercase();
        assert!(
            input.contains(substr),
            "expected \"{input}\" to contain \"{substr}\""
        )
    }
}
