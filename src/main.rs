use lox_rs::{config::RunMode, run_repl};
use std::{
    env::{args, current_dir},
    process::exit,
};

use lox_rs::{config::Config, run_file};

// use crate::{evaluate::evaluate, parser::parse, reader::read_source, tokenize::tokenize};

fn main() {
    let config = Config::build(args()).unwrap_or_else(|err| {
        eprintln!("\nERR: {err:?}");
        exit(64);
    });

    match config.mode {
        RunMode::Repl => {
            run_repl();
        }
        RunMode::Source(path) => {
            if let Err(e) = run_file(&path) {
                eprintln!(
                    "Error running program \"{}/{path}\"",
                    current_dir()
                        .expect("unable to get current directory??")
                        .to_str()
                        .unwrap()
                );
                eprintln!("\n{e}");
                exit(1);
            };
        }
    };
}
