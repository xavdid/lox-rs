use std::fmt::Display;

use crate::scanner::TokenType;

#[derive(Debug, PartialEq)]
pub struct Ast {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Number(f64),
    String(String),
    True,
    False,
    Nil,
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            Literal::Number(n) => n.to_string(),
            Literal::String(s) => format!("\"{s}\""),
            Literal::True => "true".to_string(),
            Literal::False => "false".to_string(),
            Literal::Nil => "nil".to_string(),
        };
        write!(f, "{res}")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        };
        write!(f, "{res}")
    }
}
impl From<TokenType> for UnaryOp {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::Bang => UnaryOp::Not,
            TokenType::Minus => UnaryOp::Neg,
            _ => panic!("invalid conversion to unaryop from token: {value:?}"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Add,
    Sub,
    Mul,
    Div,
}

impl From<TokenType> for BinaryOp {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::Plus => BinaryOp::Add,
            TokenType::Minus => BinaryOp::Sub,
            TokenType::Star => BinaryOp::Mul,
            TokenType::Slash => BinaryOp::Div,
            TokenType::EqualEqual => BinaryOp::Eq,
            TokenType::BangEqual => BinaryOp::Ne,
            TokenType::GreaterThan => BinaryOp::Gt,
            TokenType::GreaterThanEqual => BinaryOp::Gte,
            TokenType::LessThan => BinaryOp::Lt,
            TokenType::LessThanEqual => BinaryOp::Lte,
            _ => panic!("invalid conversion to binaryop from token: {value:?}"),
        }
    }
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Lte => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Gte => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
        };
        write!(f, "{res}")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    // TODO: store information about where this came from? like line? in the evaluator we just know something is wrong)
    Literal(Literal),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Grouping(Box<Expr>),
    /** variable read by name */
    Variable(String),
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            Expr::Literal(literal) => literal.to_string(),
            Expr::Binary { left, op, right } => format!("{left} {op} {right}"),
            Expr::Unary { op, expr } => format!("{op}{expr}"),
            Expr::Grouping(expr) => format!("({expr})"),
            Expr::Variable(_) => todo!(),
        };
        write!(f, "{res}")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    // TODO: store information about where this came from? like line? in the evaluator we just know something is wrong)
    Expression(Expr),
    Print(Expr),
    /** variable declaration (w/ optional initialization) */
    Var {
        name: String,
        val: Option<Expr>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_printing() {
        assert_eq!(
            Expr::Literal(Literal::String("cool".to_string())).to_string(),
            "\"cool\"".to_string()
        );
        assert_eq!(
            Expr::Literal(Literal::Number(123.4)).to_string(),
            "123.4".to_string()
        );
        assert_eq!(Expr::Literal(Literal::True).to_string(), "true".to_string());
        assert_eq!(
            Expr::Literal(Literal::False).to_string(),
            "false".to_string()
        );
        assert_eq!(Expr::Literal(Literal::Nil).to_string(), "nil".to_string());

        assert_eq!(
            Expr::Binary {
                left: Expr::Literal(Literal::Number(1.0)).into(),
                op: BinaryOp::Add,
                right: Expr::Literal(Literal::Number(2.0)).into()
            }
            .to_string(),
            "1 + 2".to_string()
        );

        assert_eq!(
            Expr::Binary {
                left: Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Literal(Literal::Number(1.0)).into()
                }
                .into(),
                op: BinaryOp::Sub,
                right: Expr::Literal(Literal::Number(2.0)).into()
            }
            .to_string(),
            "-1 - 2".to_string()
        );

        assert_eq!(
            Expr::Binary {
                left: Expr::Binary {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: BinaryOp::Mul,
                    right: Expr::Literal(Literal::Number(3.0)).into()
                }
                .into(),
                op: BinaryOp::Div,
                right: Expr::Literal(Literal::Number(2.0)).into()
            }
            .to_string(),
            "1 * 3 / 2".to_string()
        );

        assert_eq!(
            Expr::Binary {
                left: Expr::Binary {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: BinaryOp::Lte,
                    right: Expr::Literal(Literal::Number(3.0)).into()
                }
                .into(),
                op: BinaryOp::Gt,
                right: Expr::Literal(Literal::Number(2.0)).into()
            }
            .to_string(),
            "1 <= 3 > 2".to_string()
        );

        assert_eq!(
            Expr::Binary {
                left: Expr::Grouping(
                    Expr::Binary {
                        left: Expr::Literal(Literal::Number(1.0)).into(),
                        op: BinaryOp::Eq,
                        right: Expr::Literal(Literal::Number(3.0)).into()
                    }
                    .into()
                )
                .into(),
                op: BinaryOp::Ne,
                right: Expr::Literal(Literal::Number(2.0)).into()
            }
            .to_string(),
            "(1 == 3) != 2".to_string()
        );

        assert_eq!(
            Expr::Binary {
                left: Expr::Grouping(
                    Expr::Binary {
                        left: Expr::Literal(Literal::False).into(),
                        op: BinaryOp::Lt,
                        right: Expr::Literal(Literal::Nil).into()
                    }
                    .into()
                )
                .into(),
                op: BinaryOp::Gte,
                right: Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Expr::Literal(Literal::Number(2.0)).into()
                }
                .into()
            }
            .to_string(),
            // doesn't make sense, but is valid
            "(false < nil) >= !2".to_string()
        );
    }
}
