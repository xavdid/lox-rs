use crate::tokenize::Tokens;

pub struct Ast {}

pub fn parse(tokens: Tokens) -> Ast {
    println!("Parsing!");
    Ast {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        parse(Tokens {});
    }
}
