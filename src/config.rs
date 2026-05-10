use anyhow::{Result, anyhow};

#[derive(Debug, PartialEq)]
pub enum RunMode {
    Repl,
    Source(String),
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub mode: RunMode,
}

impl Config {
    /** Takes env:args and builds a config out of them. Fails if a filepath isn't given */
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config> {
        // get rid of program name
        args.next();

        // valid to call with 0 or 1 args
        let config = match args.next() {
            Some(arg) => Config {
                mode: RunMode::Source(arg),
            },
            None => Config {
                mode: RunMode::Repl,
            },
        };

        // but invalid with 2+
        match args.next() {
            Some(_) => Err(anyhow!("Too many args")),
            None => Ok(config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repl_mode_works() {
        assert_eq!(
            Config::build(vec!["my-program".to_string()].into_iter()).unwrap(),
            Config {
                mode: RunMode::Repl
            }
        );
    }

    #[test]
    fn source_mode_works() {
        assert_eq!(
            Config::build(vec!["my-program".to_string(), "a/b/c".to_string()].into_iter()).unwrap(),
            Config {
                mode: RunMode::Source("a/b/c".to_string())
            }
        );
    }

    #[test]
    fn it_fails_with_extra_args() {
        assert_eq!(
            Config::build(
                vec![
                    "my-program".to_string(),
                    "a/b/c".to_string(),
                    "--whatever".to_string()
                ]
                .into_iter()
            )
            .unwrap_err()
            .to_string()
            .to_lowercase(),
            "too many args"
        );
    }
}
