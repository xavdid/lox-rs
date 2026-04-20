use crate::reader::Source;

pub struct Tokens {}

pub fn tokenize(source: Source) -> Tokens {
    println!("Tokenizing!");
    Tokens {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        tokenize(Source {});
    }
}
