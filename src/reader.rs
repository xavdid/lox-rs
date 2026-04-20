use std::{fs::read_to_string, io};

pub struct Source {
    pub text: String,
}

pub fn read_source(path: &str) -> Result<Source, io::Error> {
    println!("Reading source!");

    let text = read_to_string(path)?;

    Ok(Source { text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // file in project root, which is where the tests are run from
        read_source("Cargo.toml").unwrap();
    }
}
