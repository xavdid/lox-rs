use std::{
    fmt::Display,
    io::{self, Write},
    process::exit,
};

use crate::{
    interpreter::{InterpreterError, interpret},
    parser::{ParserError, parse},
    reader::{Source, read_source},
    scanner::{ScannerError, tokenize},
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

impl From<io::Error> for LoxError {
    fn from(value: io::Error) -> Self {
        LoxError::Reader(value.to_string())
    }
}

impl From<Vec<ScannerError>> for LoxError {
    fn from(value: Vec<ScannerError>) -> Self {
        LoxError::Scanner(
            value
                .iter()
                .map(|e| format!("Scanner err: {e}\n"))
                .collect(),
        )
    }
}
impl From<Vec<ParserError>> for LoxError {
    fn from(value: Vec<ParserError>) -> Self {
        LoxError::Parser(value.iter().map(|e| format!("\n{e}")).collect())
    }
}
impl From<InterpreterError> for LoxError {
    fn from(value: InterpreterError) -> Self {
        LoxError::Interpreter(format!("{value}"))
    }
}

pub fn run_file(file_path: &str) -> Result<(), LoxError> {
    let source = read_source(file_path)?;
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

        if let Err(e) = run(&Source {
            text: input.to_string(),
        }) {
            eprintln!("{e}")
        }
    }
}

// TODO: this should probably return something? for testability?
pub fn run(input: &Source) -> Result<(), LoxError> {
    // this is the core of the interpreter
    let tokens = tokenize(input)?;
    let ast = parse(tokens)?;
    interpret(ast)?;

    Ok(())
}
