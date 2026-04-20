use crate::parser::Ast;

pub fn evaluate(_ast: Ast) {
    println!("Evaluating!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        evaluate(Ast {});
    }
}
