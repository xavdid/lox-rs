use crate::tokenize::Tokens;

pub type Ast = ();

pub fn parse(tokens: Tokens) -> Ast {
    println!("Parsing!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        parse(());
    }
}
