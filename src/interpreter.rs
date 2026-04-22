use std::fmt::Display;

use crate::ast::*;

#[derive(PartialEq, Clone, Debug)]
pub enum LoxValue {
    Number(f64),
    LString(String),
    Boolean(bool),
    Nil,
}

#[derive(Debug, PartialEq)]
pub enum InterpreterError {
    InvalidUnaryExpr(Expr),
    InvalidBinaryExpr(Expr, LoxValue, LoxValue),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::InvalidUnaryExpr(expr) => match expr {
                Expr::Unary { .. } => write!(f, "Invalid unary: {expr}."),
                _ => panic!("Put a non-unary expression in a unary error: {expr}"),
            },
            InterpreterError::InvalidBinaryExpr(expr, left, right) => match expr {
                Expr::Binary { .. } => write!(
                    f,
                    "Expected both sides of a binary expression to have the same type. Got: {left:?} and {right:?}.",
                ),
                _ => panic!("Put a non-binary expression in a binary error: {expr}"),
            },
        }
    }
}

pub fn evaluate(expr: &Expr) -> Result<LoxValue, InterpreterError> {
    Ok(match expr {
        Expr::Literal(literal) => match literal {
            Literal::Number(n) => LoxValue::Number(*n),
            Literal::String(s) => LoxValue::LString(s.to_owned()),
            Literal::True => LoxValue::Boolean(true),
            Literal::False => LoxValue::Boolean(false),
            Literal::Nil => LoxValue::Nil,
        },
        Expr::Binary {
            left: left_raw,
            op,
            right: right_raw,
        } => {
            let left = evaluate(left_raw)?;
            let right = evaluate(right_raw)?;

            use BinaryOp::*;
            use LoxValue::*;
            match (&left, op, &right) {
                // math
                (Number(l), Add, Number(r)) => Number(l + r),
                (Number(l), Sub, Number(r)) => Number(l - r),
                (Number(l), Mul, Number(r)) => Number(l * r),
                (Number(l), Div, Number(r)) => Number(l / r),

                // comparisons
                (Number(l), Lt, Number(r)) => Boolean(l < r),
                (Number(l), Lte, Number(r)) => Boolean(l <= r),
                (Number(l), Gt, Number(r)) => Boolean(l > r),
                (Number(l), Gte, Number(r)) => Boolean(l >= r),

                // string concat
                (LString(l), Add, LString(r)) => LString(l.to_string() + r),

                _ => {
                    return Err(InterpreterError::InvalidBinaryExpr(
                        expr.clone(),
                        left.clone(),
                        right.clone(),
                    ));
                }
            }
        }
        Expr::Unary { op, expr: ex } => {
            let right = evaluate(ex)?;
            match (op, right) {
                // only numbers can be negated
                (UnaryOp::Neg, LoxValue::Number(n)) => LoxValue::Number(-n),

                // but everythign has a truthiness
                (UnaryOp::Not, LoxValue::Boolean(b)) => LoxValue::Boolean(!b),
                (UnaryOp::Not, LoxValue::Nil) => LoxValue::Boolean(true),
                // all numbers and strings are truthy
                (UnaryOp::Not, _) => LoxValue::Boolean(false),

                _ => return Err(InterpreterError::InvalidUnaryExpr(expr.clone())),
            }
        }
        Expr::Grouping(expr) => evaluate(expr)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_evaluates_literals() {
        assert_eq!(
            evaluate(&Expr::Literal(Literal::Number(123.456))),
            Ok(LoxValue::Number(123.456))
        );
        assert_eq!(
            evaluate(&Expr::Literal(Literal::String("david!".to_string()))),
            Ok(LoxValue::LString("david!".to_string()))
        );
        assert_eq!(
            evaluate(&Expr::Literal(Literal::True)),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Literal(Literal::False)),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(evaluate(&Expr::Literal(Literal::Nil)), Ok(LoxValue::Nil));
    }

    #[test]
    fn it_evaluates_groupings() {
        assert_eq!(
            evaluate(&Expr::Grouping(
                Expr::Literal(Literal::Number(123.456)).into()
            )),
            Ok(LoxValue::Number(123.456))
        );
    }

    #[test]
    fn it_inverts_unary_numbers() {
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            Ok(LoxValue::Number(-123.456))
        );

        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            Ok(LoxValue::Number(123.456))
        );
    }

    #[test]
    fn it_negates_unary_values() {
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::String("very cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::True).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::False).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Nil).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
    }

    #[test]
    fn it_handles_binary_expressions() {
        use crate::ast::Literal::*;
        use BinaryOp::*;
        use Expr::*;

        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(123.456)).into(),
                op: Add,
                right: Literal(Number(123.456)).into()
            }),
            Ok(LoxValue::Number(246.912))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Sub,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(2.0))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Mul,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(15.0))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Div,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(2.0))
        );

        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(123.456)).into(),
                op: Gt,
                right: Literal(Number(123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Gte,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Lt,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Lte,
                right: Literal(Number(6.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );

        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Add,
                right: Literal(String(" cool".to_string())).into()
            }),
            Ok(LoxValue::LString("very cool".to_string()))
        );
    }

    #[test]
    fn it_fails_to_evaluate_invalid_unary_expressions() {
        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::String("bad".to_string())).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(format!("{}", err.unwrap_err()), "Invalid unary: -\"bad\".");

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::False).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(format!("{}", err.unwrap_err()), "Invalid unary: -false.");

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::Nil).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(format!("{}", err.unwrap_err()), "Invalid unary: -nil.");
    }

    #[test]
    fn it_fails_to_evaluate_invalid_binary_expressions() {
        // str - num
        let expr = Expr::Binary {
            left: Expr::Literal(Literal::String("bad".to_string())).into(),
            op: BinaryOp::Add,
            right: Expr::Literal(Literal::Number(123.0)).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(
            err,
            Err(InterpreterError::InvalidBinaryExpr(
                expr,
                LoxValue::LString("bad".to_string()),
                LoxValue::Number(123.0)
            ))
        );
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "Expected both sides of a binary expression to have the same type. Got: LString(\"bad\") and Number(123.0)."
        );

        // str - str
        let expr = Expr::Binary {
            left: Expr::Literal(Literal::String("bad".to_string())).into(),
            op: BinaryOp::Sub,
            right: Expr::Literal(Literal::String("bad".to_string())).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(
            err,
            Err(InterpreterError::InvalidBinaryExpr(
                expr,
                LoxValue::LString("bad".to_string()),
                LoxValue::LString("bad".to_string()),
            ))
        );
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "Expected both sides of a binary expression to have the same type. Got: LString(\"bad\") and LString(\"bad\")."
        );
    }
}
