use std::{collections::HashMap, fmt::Display, mem::discriminant};

use crate::ast::*;

#[derive(PartialEq, Clone, Debug)]
pub enum LoxValue {
    Number(f64),
    LString(String),
    Boolean(bool),
    Nil,
}

impl Display for LoxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            LoxValue::Nil => "nil",
            LoxValue::Number(v) => &v.to_string(),
            LoxValue::LString(v) => v,
            LoxValue::Boolean(v) => &v.to_string(),
        };
        write!(f, "{res}")
    }
}

#[allow(clippy::enum_variant_names)] // unless we never get an invalid thing?
#[derive(Debug, PartialEq)]
pub enum InterpreterError {
    InvalidUnaryExpr(Expr),
    InvalidBinaryExprIncompatibleTypes(Expr),
    InvalidBinaryExprInvalidTypes(Expr),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::InvalidUnaryExpr(expr) => match expr {
                Expr::Unary { .. } => write!(f, "ERR: Invalid unary: {expr}."),
                _ => panic!("Put a non-unary expression in a unary error: {expr}"),
            },
            InterpreterError::InvalidBinaryExprIncompatibleTypes(expr) => match expr {
                Expr::Binary { .. } => write!(
                    f,
                    "ERR: Expected both sides of a binary expression to have the same type. Got: {expr}.",
                ),
                _ => panic!("Put a non-binary expression in a binary error: {expr}"),
            },
            InterpreterError::InvalidBinaryExprInvalidTypes(expr) => match expr {
                Expr::Binary { .. } => write!(
                    f,
                    "ERR: Operation not supported for these data types. Got: {expr}.",
                ),
                _ => panic!("Put a non-binary expression in a binary error: {expr}"),
            },
        }
    }
}

/** represents a scope */
pub struct Environment {
    // values: HashMap<String, LoxValue>, // Rc<RefCell<Hashmap<>>>?
}

impl Environment {
    pub fn get(&self, name: &String) -> Option<LoxValue> {
        todo!("implement getting vars")
    }
    pub fn set(&mut self, name: &String, value: &LoxValue) {
        todo!("implement setting vars")
    }
}

pub fn interpret(ast: Ast) -> Result<(), InterpreterError> {
    for stmt in ast.statements {
        execute(&stmt)?
    }

    Ok(())
}

fn execute(stmt: &Stmt) -> Result<(), InterpreterError> {
    match stmt {
        Stmt::Expression(expr) => {
            evaluate(expr)?;
        }
        Stmt::Print(expr) => {
            let val = evaluate(expr)?;
            println!("{val}");
        }
        Stmt::Var { name, val } => todo!(),
    }

    Ok(())
}

fn evaluate(expr: &Expr) -> Result<LoxValue, InterpreterError> {
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

                // equality requires same type and value
                (l, Eq, r) => Boolean(l == r),
                (l, Ne, r) => Boolean(l != r),

                _ => {
                    return if discriminant(&left) == discriminant(&right) {
                        Err(InterpreterError::InvalidBinaryExprInvalidTypes(
                            expr.clone(),
                        ))
                    } else {
                        Err(InterpreterError::InvalidBinaryExprIncompatibleTypes(
                            expr.clone(),
                        ))
                    };
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
        Expr::Variable(_) => todo!(),
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
        assert_eq!(
            evaluate(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Expr::Literal(Literal::Number(2.0)).into()
                }
                .into()
            }),
            Ok(LoxValue::Boolean(true))
        );
    }

    #[test]
    fn it_evaluates_binary_expressions() {
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
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(String("cool".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(456.0)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(123.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(True).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(Nil).into()
            }),
            Ok(LoxValue::Boolean(true))
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
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Invalid unary: -\"bad\"."
        );

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::False).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Invalid unary: -false."
        );

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::Nil).into(),
        };
        let err = evaluate(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(format!("{}", err.unwrap_err()), "ERR: Invalid unary: -nil.");
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
            Err(InterpreterError::InvalidBinaryExprIncompatibleTypes(expr,))
        );
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Expected both sides of a binary expression to have the same type. Got: \"bad\" + 123."
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
            Err(InterpreterError::InvalidBinaryExprInvalidTypes(expr,))
        );
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Operation not supported for these data types. Got: \"bad\" - \"bad\"."
        );
    }
}
