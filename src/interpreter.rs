use std::{fmt::Display, mem::discriminant};

use crate::ast::*;
use crate::environment::Environment;

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

pub fn interpret(ast: Ast) -> Result<(), InterpreterError> {
    let env = Environment::new();
    for stmt in ast.statements {
        execute(&stmt, &env)?
    }

    Ok(())
}

/** execute a statement for its side effects */
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
        Stmt::Block(stmts) => {
            let subscope = env.child_scope();

            for s in stmts {
                execute(s, &subscope)?;
            }
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if is_truthy(&evaluate(condition, env)?) {
                execute(then_branch, env)?;
            } else if let Some(else_branch) = else_branch {
                execute(else_branch, env)?;
            }
        }
        Stmt::While { condition, body } => {
            while is_truthy(&evaluate(condition, env)?) {
                execute(body, env)?
            }
        }
    }

    Ok(())
}

/** evaluate the result of an exprsesion */
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
                (UnaryOp::Not, v) => LoxValue::Boolean(!is_truthy(&v)),

                _ => return Err(InterpreterError::InvalidUnaryExpr(expr.clone())),
            }
        }
        Expr::Grouping(expr) => evaluate(expr, env)?,
        Expr::Variable(name) => match env.get(name) {
            Some(v) => v,
            None => return Err(InterpreterError::UndefinedVariable(expr.clone())),
        },
        Expr::Assign { name, value } => {
            let val = evaluate(value, env)?;

            match env.assign(name, val) {
                Some(val) => val,
                None => return Err(InterpreterError::UndefinedVariable(expr.clone())),
            }
        }
        Expr::Logical { left, op, right } => {
            let left = evaluate(left, env)?;

            match op {
                // short circuit logic - only evaluate right if we can reach it
                LogicalOp::Or if is_truthy(&left) => return Ok(left),
                LogicalOp::And if !is_truthy(&left) => return Ok(left),
                _ => evaluate(right, env)?,
            }
        }
    })
}

fn is_truthy(expr: &LoxValue) -> bool {
    match expr {
        LoxValue::Number(_) | LoxValue::LString(_) => true,
        LoxValue::Boolean(b) => *b,
        LoxValue::Nil => false,
    }
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

    #[allow(clippy::bool_assert_comparison)]
    #[test]
    fn it_tests_truthiness() {
        assert_eq!(is_truthy(&LoxValue::Nil), false);

        assert_eq!(is_truthy(&LoxValue::Boolean(true)), true);
        assert_eq!(is_truthy(&LoxValue::Boolean(false)), false);

        assert_eq!(is_truthy(&LoxValue::Number(0.0)), true);
        assert_eq!(is_truthy(&LoxValue::Number(1.0)), true);
        assert_eq!(is_truthy(&LoxValue::Number(-1.0)), true);

        assert_eq!(is_truthy(&LoxValue::LString("david".to_string())), true);
        assert_eq!(is_truthy(&LoxValue::LString("".to_string())), true);
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
    fn it_evaulates_logical_or_truthy() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: LogicalOp::Or,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                },
                &env
            ),
            Ok(LoxValue::Number(1.0))
        );
    }

    #[test]
    fn it_evaulates_logical_or_falsy() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::Nil).into(),
                    op: LogicalOp::Or,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                },
                &env
            ),
            Ok(LoxValue::Number(2.0))
        );
    }

    #[test]
    fn it_evaulates_logical_and_truthy() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: LogicalOp::And,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                },
                &env
            ),
            Ok(LoxValue::Number(2.0))
        );
    }

    #[test]
    fn it_evaulates_logical_and_falsy() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::Nil).into(),
                    op: LogicalOp::And,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                },
                &env
            ),
            Ok(LoxValue::Nil)
        );
    }

    #[test]
    fn it_short_circuits_and() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::Nil).into(),
                    op: LogicalOp::And,
                    // this would be an error if evaluated
                    right: Expr::Binary {
                        left: Expr::Literal(Literal::Nil).into(),
                        op: BinaryOp::Add,
                        right: Expr::Literal(Literal::Nil).into()
                    }
                    .into()
                },
                &env
            ),
            Ok(LoxValue::Nil)
        );
    }

    #[test]
    fn it_short_circuits_or() {
        let env = Environment::new();

        assert_eq!(
            evaluate(
                &Expr::Logical {
                    left: Expr::Literal(Literal::True).into(),
                    op: LogicalOp::Or,
                    // this would be an error if evaluated
                    right: Expr::Binary {
                        left: Expr::Literal(Literal::Nil).into(),
                        op: BinaryOp::Add,
                        right: Expr::Literal(Literal::Nil).into()
                    }
                    .into()
                },
                &env
            ),
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

        assert_eq!(
            env.get("name"),
            Some(LoxValue::LString("david".to_string()))
        );
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

        assert_eq!(env.get("name"), Some(LoxValue::Nil));
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

        assert_eq!(
            env.get("name"),
            Some(LoxValue::LString("david".to_string()))
        );
    }

    #[test]
    fn it_evaluates_truthy_if_blocks() {
        // this is how we'll track side effects
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
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::If {
                    condition: Expr::Literal(Literal::True),
                    then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(123.0)).into()
                    })])
                    .into(),
                    else_branch: None
                },
                &env
            ),
            Ok(())
        );

        // now set!
        assert_eq!(env.get("name"), Some(LoxValue::Number(123.0)));
    }

    #[test]
    fn it_doesnt_evaluate_falsy_if_blocks() {
        // this is how we'll track side effects
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
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::If {
                    condition: Expr::Literal(Literal::False),
                    then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(123.0)).into()
                    })])
                    .into(),
                    else_branch: None
                },
                &env
            ),
            Ok(())
        );

        // still not set
        assert_eq!(env.get("name"), Some(LoxValue::Nil));
    }

    #[test]
    fn it_ignores_else_for_truthy_if_blocks() {
        // this is how we'll track side effects
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
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::If {
                    condition: Expr::Literal(Literal::True),
                    then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(123.0)).into()
                    })])
                    .into(),
                    else_branch: Some(
                        Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                            name: "name".to_string(),
                            value: Expr::Literal(Literal::Number(456.0)).into()
                        })])
                        .into()
                    )
                },
                &env
            ),
            Ok(())
        );

        // now set!
        assert_eq!(env.get("name"), Some(LoxValue::Number(123.0)));
    }

    #[test]
    fn it_evaluates_else_for_falsy_if() {
        // this is how we'll track side effects
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
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::If {
                    condition: Expr::Literal(Literal::False),
                    then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(123.0)).into()
                    })])
                    .into(),
                    else_branch: Some(
                        Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                            name: "name".to_string(),
                            value: Expr::Literal(Literal::Number(456.0)).into()
                        })])
                        .into()
                    )
                },
                &env
            ),
            Ok(())
        );

        // still not set
        assert_eq!(env.get("name"), Some(LoxValue::Number(456.0)));
    }

    #[test]
    fn it_evaluates_while_loops() {
        // this is how we'll track side effects
        let env = Environment::new();
        env.define("num", LoxValue::Number(1.0));

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::While {
                    condition: Expr::Binary {
                        left: Expr::Variable("num".to_string()).into(),
                        op: BinaryOp::Lte,
                        right: Expr::Literal(Literal::Number(3.0)).into()
                    },
                    body: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "num".to_string(),
                        value: Expr::Binary {
                            left: Expr::Variable("num".to_string()).into(),
                            op: BinaryOp::Add,
                            right: Expr::Literal(Literal::Number(1.0)).into()
                        }
                        .into()
                    })])
                    .into(),
                },
                &env
            ),
            Ok(())
        );

        // still not set
        assert_eq!(env.get("num"), Some(LoxValue::Number(4.0)));
    }

    #[test]
    fn it_creates_new_env_for_blocks() {
        let env = Environment::new();

        assert_eq!(
            execute(
                // this declaration happens in a subscope...
                &Stmt::Block(vec![Stmt::Var {
                    name: "name".to_string(),
                    val: Expr::Literal(Literal::String("david".to_string())).into(),
                }]),
                &env
            ),
            Ok(())
        );

        // ... so the variable isn't present in our root scope
        assert_eq!(env.get("name"), None);
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
