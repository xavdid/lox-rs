use std::{
    io::{self, Write},
    process::exit,
};

use anyhow::{Error, Result, anyhow};

use crate::{
    environment::Environment,
    interpreter::{env_with_globals, interpret},
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

pub fn run_file(file_path: &str) -> Result<()> {
    let source = read_source(file_path)?;
    run(&source, None, false)?;
    Ok(())
}

/// Given a bunch of Errors, turn them into one newline-separated list.
///
/// Bridges the gap between things like the parser (which can return many errors) and main, which expects a single one.
fn join_errors(errors: Vec<Error>) -> Error {
    let joined: String = errors.iter().map(|e| format!("{e}\n")).collect();
    anyhow!(joined)
}

// never returns; run until quit
pub fn run_repl() -> ! {
    // TODO: print expressions, e.g.
    // >>> 1 + 1;
    // > 2
    let env = env_with_globals();
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

        if let Err(e) = run(
            &Source {
                text: input.to_string(),
            },
            Some(&env),
            true,
        ) {
            eprintln!("{e}")
        }
    }
}

fn run(input: &Source, env: Option<&Environment>, print_values: bool) -> Result<()> {
    // this is the core of the interpreter
    let tokens = tokenize(input)?;
    let ast = parse(tokens)?;
    interpret(ast, env, print_values)?;

    Ok(())
}

#[cfg(test)]
pub mod test_util {
    use anyhow::Error;

    /// test helper for making assertions about error messages. Performs a case-insensitive match on the error message
    pub fn assert_contains(err: &Error, msg: &str) {
        let input = err.to_string().to_lowercase();
        let substr = &msg.to_lowercase();
        assert!(
            input.contains(substr),
            "expected \"{input}\" to contain \"{substr}\""
        )
    }
}
