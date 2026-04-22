use crate::ast::Ast;

pub fn evaluate(_ast: Ast) {
    println!("Evaluating!");
}

#[cfg(test)]
mod tests {

    use crate::ast::Expr;

    use super::*;

    #[test]
    fn it_works() {
        evaluate(Ast {
            root: Expr::Literal(crate::ast::Literal::Nil),
        });
    }
}
