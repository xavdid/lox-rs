use lox_rs::{config::RunMode, run_repl};
use std::{env::args, process::exit};

use lox_rs::{config::Config, run_file};

// use crate::{evaluate::evaluate, parser::parse, reader::read_source, tokenize::tokenize};

fn main() {
    let config = match Config::build(args()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err:?}");
            exit(64);
        }
    };

    match config.mode {
        RunMode::Repl => run_repl(),
        RunMode::Source(_) => match run_file(&config) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{e:?}");
                exit(1);
            }
        },
    };
}
