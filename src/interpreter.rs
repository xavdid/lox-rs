use std::{cell::RefCell, collections::HashMap, fmt::Display, mem::discriminant, rc::Rc};

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
    UndefinedVariable(Expr),
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
            InterpreterError::UndefinedVariable(expr) => {
                write!(f, "ERR: Variable {expr} not defined.",)
            }
        }
    }
}

/** represents a scope */
pub struct Environment {
    // Rc means I can have shared ownership of the object, useful when copying environments between scopes
    // RefCell provies "interior mutability", meaning `env` doesn't have to be mutably borrowed everywhere and .
    //   ownership checks are deferred to runtime instead of compile time
    //   in exchange, multiple owners can all try to mutate the data (as long as they follow the normal rules; it's a panic if there are two mutable borrows at once)
    values: Rc<RefCell<HashMap<String, LoxValue>>>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            values: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn get(&self, name: &str) -> Result<LoxValue, bool> {
        self.values
            .borrow()
            .get(name)
            .cloned() // TODO: this means we clone on every variable read, which isn't great!
            .ok_or(false)
    }

    // set for the first time
    // var a = 3;
    pub fn define(&self, name: &str, value: LoxValue) {
        self.values.borrow_mut().insert(name.into(), value);
    }

    // updates an existing variable, but can't create
    // var a; a = 3; // ok
    // b = 3; // err, `b` is not defined
    pub fn assign(&self, name: &str, value: LoxValue) -> Result<LoxValue, bool> {
        if self.values.borrow().contains_key(name) {
            self.values.borrow_mut().insert(name.into(), value.clone());
            Ok(value)
        } else {
            Err(false) // these bools are a code smell
        }
    }
}

pub fn interpret(ast: Ast) -> Result<(), InterpreterError> {
    let env = Environment::new();
    for stmt in ast.statements {
        execute(&stmt, &env)?
    }

    Ok(())
}

fn execute(stmt: &Stmt, env: &Environment) -> Result<(), InterpreterError> {
    match stmt {
        Stmt::Expression(expr) => {
            evaluate(expr, env)?;
        }
        Stmt::Print(expr) => {
            let val = evaluate(expr, env)?;
            println!("{val}");
        }
        Stmt::Var { name, val } => {
            env.define(
                name,
                match val {
                    Some(expr) => evaluate(expr, env)?,
                    None => LoxValue::Nil,
                },
            );
        }
    }

    Ok(())
}

fn evaluate(expr: &Expr, env: &Environment) -> Result<LoxValue, InterpreterError> {
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
            let left = evaluate(left_raw, env)?;
            let right = evaluate(right_raw, env)?;

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
            let right = evaluate(ex, env)?;
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
        Expr::Grouping(expr) => evaluate(expr, env)?,
        Expr::Variable(name) => match env.get(name) {
            Ok(v) => v,
            Err(_) => return Err(InterpreterError::UndefinedVariable(expr.clone())),
        },
        Expr::Assign { name, value } => {
            let val = evaluate(value, env)?;

            match env.assign(name, val.clone()) {
                Ok(_) => val,
                Err(_) => return Err(InterpreterError::UndefinedVariable(expr.clone())),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper for tests
    fn test_eval(e: &Expr) -> Result<LoxValue, InterpreterError> {
        let env = Environment::new();
        evaluate(e, &env)
    }

    #[test]
    fn it_evaluates_literals() {
        assert_eq!(
            test_eval(&Expr::Literal(Literal::Number(123.456))),
            Ok(LoxValue::Number(123.456))
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::String("david!".to_string()))),
            Ok(LoxValue::LString("david!".to_string()))
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::True)),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::False)),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(test_eval(&Expr::Literal(Literal::Nil)), Ok(LoxValue::Nil));
    }

    #[test]
    fn it_evaluates_groupings() {
        assert_eq!(
            test_eval(&Expr::Grouping(
                Expr::Literal(Literal::Number(123.456)).into()
            )),
            Ok(LoxValue::Number(123.456))
        );
    }

    #[test]
    fn it_inverts_unary_numbers() {
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            Ok(LoxValue::Number(-123.456))
        );

        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            Ok(LoxValue::Number(123.456))
        );
    }

    #[test]
    fn it_negates_unary_values() {
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::String("very cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::True).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::False).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Nil).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Unary {
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
            test_eval(&Expr::Binary {
                left: Literal(Number(123.456)).into(),
                op: Add,
                right: Literal(Number(123.456)).into()
            }),
            Ok(LoxValue::Number(246.912))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Sub,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(2.0))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Mul,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(15.0))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Div,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Number(2.0))
        );

        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.456)).into(),
                op: Gt,
                right: Literal(Number(123.456)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Gte,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Lt,
                right: Literal(Number(3.0)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Lte,
                right: Literal(Number(6.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );

        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Add,
                right: Literal(String(" cool".to_string())).into()
            }),
            Ok(LoxValue::LString("very cool".to_string()))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("cool".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(456.0)).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(123.0)).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(True).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            Ok(LoxValue::Boolean(false))
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(Nil).into()
            }),
            Ok(LoxValue::Boolean(true))
        );
    }

    #[test]
    fn it_evaluates_present_variables() {
        let env = Environment::new();
        env.define("name", LoxValue::LString("david".to_string()));

        assert_eq!(
            evaluate(&Expr::Variable("name".to_string()), &env),
            Ok(LoxValue::LString("david".to_string()))
        );
    }

    #[test]
    fn it_declares_values() {
        let env = Environment::new();

        assert_eq!(
            execute(
                &Stmt::Var {
                    name: "name".to_string(),
                    val: Expr::Literal(Literal::String("david".to_string())).into()
                },
                &env
            ),
            Ok(())
        );

        assert_eq!(env.get("name"), Ok(LoxValue::LString("david".to_string())));
    }

    #[test]
    fn it_initializes_vars_to_nil() {
        let env = Environment::new();

        assert_eq!(
            execute(
                &Stmt::Var {
                    name: "name".to_string(),
                    val: None
                },
                &env
            ),
            Ok(())
        );

        assert_eq!(env.get("name"), Ok(LoxValue::Nil));
    }

    #[test]
    fn it_assigns_values() {
        let env = Environment::new();

        assert_eq!(
            execute(
                &Stmt::Var {
                    name: "name".to_string(),
                    val: None
                },
                &env
            ),
            Ok(())
        );
        assert_eq!(
            execute(
                &Stmt::Expression(Expr::Assign {
                    name: "name".to_string(),
                    value: Expr::Literal(Literal::String("david".to_string())).into()
                }),
                &env
            ),
            Ok(())
        );

        assert_eq!(env.get("name"), Ok(LoxValue::LString("david".to_string())));
    }

    #[test]
    fn it_fails_to_read_missing_variables() {
        let expr = Expr::Variable("name".to_string());
        assert_eq!(
            test_eval(&expr),
            Err(InterpreterError::UndefinedVariable(expr))
        );
    }

    #[test]
    fn it_fails_to_evaluate_invalid_unary_expressions() {
        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::String("bad".to_string())).into(),
        };
        let err = test_eval(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Invalid unary: -\"bad\"."
        );

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::False).into(),
        };
        let err = test_eval(&expr);
        assert_eq!(err, Err(InterpreterError::InvalidUnaryExpr(expr)));
        assert_eq!(
            format!("{}", err.unwrap_err()),
            "ERR: Invalid unary: -false."
        );

        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::Nil).into(),
        };
        let err = test_eval(&expr);
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
        let err = test_eval(&expr);
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
        let err = test_eval(&expr);
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
