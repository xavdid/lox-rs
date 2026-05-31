use anyhow::{Result, anyhow};
use std::{fmt::Display, mem::discriminant};

use crate::ast::*;
use crate::environment::Environment;

trait IsCallable {
    // could be used for native functions?
    fn call(&self, env: &Environment, arguments: Vec<LoxValue>) -> LoxValue;
}

#[derive(PartialEq, Clone, Debug)]
pub struct InnerCallable {
    declaration: Stmt,
    arity: usize,
    // TODO: global functions
}

// impl IsCallable for Callable {
impl InnerCallable {
    fn call(&self, globals: &Environment, arguments: Vec<LoxValue>) -> Result<LoxValue> {
        let (parameters, body) = match &self.declaration {
            Stmt::Function {
                parameters, body, ..
            } => (parameters, body),
            _ => panic!(
                "Expected declaration to be a Stmt::Function, got {:?}",
                self.declaration
            ),
        };

        let env = globals.child_scope();
        for (idx, val) in arguments.iter().enumerate() {
            env.define(&parameters[idx], val.clone()); // TODO: need to clone?
        }

        execute(body, &env)?;
        // TODO: return a value? I guess that's later
        Ok(LoxValue::Nil)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum LoxValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Callable(InnerCallable),
    Nil,
}

impl Display for LoxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            LoxValue::Nil => "nil",
            LoxValue::Number(v) => &v.to_string(),
            LoxValue::String(v) => &format!("\"{v}\""),
            LoxValue::Boolean(v) => &v.to_string(),
            LoxValue::Callable(_) => "<native fn>",
        };
        write!(f, "{res}")
    }
}

pub fn interpret(ast: Ast) -> Result<()> {
    let globals = Environment::new();

    // globals.define(
    //     "clock",
    //     LoxValue::Callable(Callable {
    //         arity: 0,
    //         name: "clock".to_string(),
    //         // TODO: global functions
    //         // not sure how to type this
    //         // defn: |env: &Environment, args: Vec<LoxValue>| Instant::now(),
    //     }),
    // );

    for stmt in ast.statements {
        execute(&stmt, &globals)?
    }

    Ok(())
}

/// execute a statement for its side effects
fn execute(stmt: &Stmt, env: &Environment) -> Result<()> {
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
        Stmt::Function {
            name, parameters, ..
        } => {
            env.define(
                name,
                LoxValue::Callable(InnerCallable {
                    // capture the whole function, not just the block
                    declaration: stmt.clone(), // TODO: not positive this is right
                    arity: parameters.len(),
                }),
            );
        }
    }

    Ok(())
}

/// evaluate the result of an exprsesion
fn evaluate(expr: &Expr, env: &Environment) -> Result<LoxValue> {
    Ok(match expr {
        Expr::Literal(literal) => match literal {
            Literal::Number(n) => LoxValue::Number(*n),
            Literal::String(s) => LoxValue::String(s.to_owned()),
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
                (String(l), Add, String(r)) => String(l.to_string() + r),

                // equality requires same type and value
                (l, Eq, r) => Boolean(l == r),
                (l, Ne, r) => Boolean(l != r),

                _ => {
                    return if discriminant(&left) == discriminant(&right) {
                        Err(anyhow!(
                            "Operation \"{op}\" not supported between {left:?} and {right:?}."
                        ))
                    } else {
                        Err(anyhow!(
                            "Expected both sides of a binary expression to have the same type, got {left:?} and {right:?}."
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

                _ => return Err(anyhow!("Invalid unary: {expr}")),
            }
        }
        Expr::Grouping(expr) => evaluate(expr, env)?,
        Expr::Variable(name) => match env.get(name) {
            Some(v) => v,
            None => return Err(anyhow!("Variable \"{expr}\" not defined.")),
        },
        Expr::Assign { name, value } => {
            let val = evaluate(value, env)?;

            match env.assign(name, val) {
                Some(val) => val,
                None => return Err(anyhow!("Variable \"{expr}\" not defined.")),
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
        Expr::Call { callee, arguments } => {
            let func = evaluate(callee, env)?;

            let callable = match func {
                LoxValue::Callable(c) => c,
                _ => {
                    return Err(anyhow!(
                        "{func:?} is not callable; Can only call functions and classes"
                    ));
                }
            };

            // evaulate before checking arity, since we may need the side effects
            // this is what python done
            let args = arguments
                .iter()
                .map(|e| evaluate(e, env))
                .collect::<Result<Vec<_>, _>>()?;

            if args.len() != callable.arity {
                return Err(anyhow!(
                    "Expected {} arg(s) but got {}",
                    callable.arity,
                    args.len()
                ));
            }

            callable.call(env, args)?
        }
    })
}

fn is_truthy(expr: &LoxValue) -> bool {
    match expr {
        LoxValue::Boolean(b) => *b,
        LoxValue::Nil => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_contains;
    use pretty_assertions::assert_eq;

    // Helper for tests
    fn test_eval(e: &Expr) -> LoxValue {
        let env = Environment::new();
        evaluate(e, &env).expect("eval should return Ok(())")
    }

    fn fail_eval(e: &Expr) -> anyhow::Error {
        let env = Environment::new();
        evaluate(e, &env).expect_err("fail_eval should return Err(...)")
    }

    #[test]
    fn it_evaluates_literals() {
        assert_eq!(
            test_eval(&Expr::Literal(Literal::Number(123.456))),
            LoxValue::Number(123.456)
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::String("david!".to_string()))),
            LoxValue::String("david!".to_string())
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::True)),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Literal(Literal::False)),
            LoxValue::Boolean(false)
        );
        assert_eq!(test_eval(&Expr::Literal(Literal::Nil)), LoxValue::Nil);
    }

    #[test]
    fn it_evaluates_groupings() {
        assert_eq!(
            test_eval(&Expr::Grouping(
                Expr::Literal(Literal::Number(123.456)).into()
            )),
            LoxValue::Number(123.456)
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

        assert_eq!(is_truthy(&LoxValue::String("david".to_string())), true);
        assert_eq!(is_truthy(&LoxValue::String("".to_string())), true);
    }

    #[test]
    fn it_inverts_unary_numbers() {
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            LoxValue::Number(-123.456)
        );

        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Neg,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            LoxValue::Number(123.456)
        );
    }

    #[test]
    fn it_negates_unary_values() {
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(123.456)).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Number(-123.456)).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::String("very cool".to_string())).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::True).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::False).into()
            }),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Unary {
                op: UnaryOp::Not,
                expr: Expr::Literal(Literal::Nil).into()
            }),
            LoxValue::Boolean(true)
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
            LoxValue::Boolean(true)
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
            LoxValue::Number(246.912)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Sub,
                right: Literal(Number(3.0)).into()
            }),
            LoxValue::Number(2.0)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Mul,
                right: Literal(Number(3.0)).into()
            }),
            LoxValue::Number(15.0)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Div,
                right: Literal(Number(3.0)).into()
            }),
            LoxValue::Number(2.0)
        );

        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.456)).into(),
                op: Gt,
                right: Literal(Number(123.456)).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Gte,
                right: Literal(Number(3.0)).into()
            }),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(5.0)).into(),
                op: Lt,
                right: Literal(Number(3.0)).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(6.0)).into(),
                op: Lte,
                right: Literal(Number(6.0)).into()
            }),
            LoxValue::Boolean(true)
        );

        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Add,
                right: Literal(String(" cool".to_string())).into()
            }),
            LoxValue::String("very cool".to_string())
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("very".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(String("cool".to_string())).into(),
                op: Eq,
                right: Literal(String("cool".to_string())).into()
            }),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(456.0)).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Number(123.0)).into(),
                op: Eq,
                right: Literal(Number(123.0)).into()
            }),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(True).into(),
                op: Eq,
                right: Literal(True).into()
            }),
            LoxValue::Boolean(true)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(False).into()
            }),
            LoxValue::Boolean(false)
        );
        assert_eq!(
            test_eval(&Expr::Binary {
                left: Literal(Nil).into(),
                op: Eq,
                right: Literal(Nil).into()
            }),
            LoxValue::Boolean(true)
        );
    }

    #[test]
    fn it_evaulates_logical_or_truthy() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::Number(1.0)).into(),
                op: LogicalOp::Or,
                right: Expr::Literal(Literal::Number(2.0)).into()
            },),
            LoxValue::Number(1.0)
        );
    }

    #[test]
    fn it_evaulates_logical_or_falsy() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::Nil).into(),
                op: LogicalOp::Or,
                right: Expr::Literal(Literal::Number(2.0)).into()
            },),
            LoxValue::Number(2.0)
        );
    }

    #[test]
    fn it_evaulates_logical_and_truthy() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::Number(1.0)).into(),
                op: LogicalOp::And,
                right: Expr::Literal(Literal::Number(2.0)).into()
            },),
            LoxValue::Number(2.0)
        );
    }

    #[test]
    fn it_evaulates_logical_and_falsy() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::Nil).into(),
                op: LogicalOp::And,
                right: Expr::Literal(Literal::Number(2.0)).into()
            },),
            LoxValue::Nil
        );
    }

    #[test]
    fn it_short_circuits_and() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::Nil).into(),
                op: LogicalOp::And,
                // this would be an error if evaluated
                right: Expr::Binary {
                    left: Expr::Literal(Literal::Nil).into(),
                    op: BinaryOp::Add,
                    right: Expr::Literal(Literal::Nil).into()
                }
                .into()
            },),
            LoxValue::Nil
        );
    }

    #[test]
    fn it_short_circuits_or() {
        assert_eq!(
            test_eval(&Expr::Logical {
                left: Expr::Literal(Literal::True).into(),
                op: LogicalOp::Or,
                // this would be an error if evaluated
                right: Expr::Binary {
                    left: Expr::Literal(Literal::Nil).into(),
                    op: BinaryOp::Add,
                    right: Expr::Literal(Literal::Nil).into()
                }
                .into()
            },),
            LoxValue::Boolean(true)
        );
    }

    #[test]
    fn it_evaluates_present_variables() {
        let env = Environment::new();
        env.define("name", LoxValue::String("david".to_string()));

        assert_eq!(
            evaluate(&Expr::Variable("name".to_string()), &env).unwrap(),
            LoxValue::String("david".to_string())
        );
    }

    #[test]
    fn it_declares_values() {
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: Expr::Literal(Literal::String("david".to_string())).into(),
            },
            &env,
        )
        .unwrap();

        assert_eq!(env.get("name"), Some(LoxValue::String("david".to_string())));
    }

    #[test]
    fn it_initializes_vars_to_nil() {
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();

        assert_eq!(env.get("name"), Some(LoxValue::Nil));
    }

    #[test]
    fn it_assigns_values() {
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();

        execute(
            &Stmt::Expression(Expr::Assign {
                name: "name".to_string(),
                value: Expr::Literal(Literal::String("david".to_string())).into(),
            }),
            &env,
        )
        .unwrap();

        assert_eq!(env.get("name"), Some(LoxValue::String("david".to_string())));
    }

    #[test]
    fn it_evaluates_truthy_if_blocks() {
        // this is how we'll track side effects
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        execute(
            // this declaration happens in a subscope...
            &Stmt::If {
                condition: Expr::Literal(Literal::True),
                then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                    name: "name".to_string(),
                    value: Expr::Literal(Literal::Number(123.0)).into(),
                })])
                .into(),
                else_branch: None,
            },
            &env,
        )
        .unwrap();

        // now set!
        assert_eq!(env.get("name"), Some(LoxValue::Number(123.0)));
    }

    #[test]
    fn it_doesnt_evaluate_falsy_if_blocks() {
        // this is how we'll track side effects
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        execute(
            // this declaration happens in a subscope...
            &Stmt::If {
                condition: Expr::Literal(Literal::False),
                then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                    name: "name".to_string(),
                    value: Expr::Literal(Literal::Number(123.0)).into(),
                })])
                .into(),
                else_branch: None,
            },
            &env,
        )
        .unwrap();

        // still not set
        assert_eq!(env.get("name"), Some(LoxValue::Nil));
    }

    #[test]
    fn it_ignores_else_for_truthy_if_blocks() {
        // this is how we'll track side effects
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();

        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        execute(
            // this declaration happens in a subscope...
            &Stmt::If {
                condition: Expr::Literal(Literal::True),
                then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                    name: "name".to_string(),
                    value: Expr::Literal(Literal::Number(123.0)).into(),
                })])
                .into(),
                else_branch: Some(
                    Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(456.0)).into(),
                    })])
                    .into(),
                ),
            },
            &env,
        )
        .unwrap();

        // now set!
        assert_eq!(env.get("name"), Some(LoxValue::Number(123.0)));
    }

    #[test]
    fn it_evaluates_else_for_falsy_if() {
        // this is how we'll track side effects
        let env = Environment::new();

        execute(
            &Stmt::Var {
                name: "name".to_string(),
                val: None,
            },
            &env,
        )
        .unwrap();
        // not set yet
        assert_eq!(env.get("name"), Some(LoxValue::Nil));

        execute(
            // this declaration happens in a subscope...
            &Stmt::If {
                condition: Expr::Literal(Literal::False),
                then_branch: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                    name: "name".to_string(),
                    value: Expr::Literal(Literal::Number(123.0)).into(),
                })])
                .into(),
                else_branch: Some(
                    Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                        name: "name".to_string(),
                        value: Expr::Literal(Literal::Number(456.0)).into(),
                    })])
                    .into(),
                ),
            },
            &env,
        )
        .unwrap();

        // still not set
        assert_eq!(env.get("name"), Some(LoxValue::Number(456.0)));
    }

    #[test]
    fn it_evaluates_while_loops() {
        // this is how we'll track side effects
        let env = Environment::new();
        env.define("num", LoxValue::Number(1.0));

        execute(
            // this declaration happens in a subscope...
            &Stmt::While {
                condition: Expr::Binary {
                    left: Expr::Variable("num".to_string()).into(),
                    op: BinaryOp::Lte,
                    right: Expr::Literal(Literal::Number(3.0)).into(),
                },
                body: Stmt::Block(vec![Stmt::Expression(Expr::Assign {
                    name: "num".to_string(),
                    value: Expr::Binary {
                        left: Expr::Variable("num".to_string()).into(),
                        op: BinaryOp::Add,
                        right: Expr::Literal(Literal::Number(1.0)).into(),
                    }
                    .into(),
                })])
                .into(),
            },
            &env,
        )
        .unwrap();

        // still not set
        assert_eq!(env.get("num"), Some(LoxValue::Number(4.0)));
    }

    #[test]
    fn it_creates_new_env_for_blocks() {
        let env = Environment::new();

        execute(
            // this declaration happens in a subscope...
            &Stmt::Block(vec![Stmt::Var {
                name: "name".to_string(),
                val: Expr::Literal(Literal::String("david".to_string())).into(),
            }]),
            &env,
        )
        .unwrap();

        // ... so the variable isn't present in our root scope
        assert_eq!(env.get("name"), None);
    }

    /// this is an example test cases that isn't working, but feels like it should be at this point.
    ///
    /// I guess I don't have a testable way to execute an AST, so I'm just recrating my `interpret` function
    #[test]
    fn it_calls_declared_functions() {
        let env = Environment::new();

        let ast = Ast {
            statements: vec![
                // declare a function
                Stmt::Function {
                    name: "sum".into(),
                    parameters: vec!["a".to_string(), "b".to_string()],
                    body: Stmt::Block(vec![Stmt::Print(Expr::Binary {
                        left: Expr::Variable("a".to_string()).into(),
                        op: BinaryOp::Add,
                        right: Expr::Variable("b".to_string()).into(),
                    })])
                    .into(),
                },
                // then call it
                Stmt::Expression(Expr::Call {
                    callee: Expr::Variable("sum".to_string()).into(),
                    arguments: vec![
                        Expr::Literal(Literal::Number(1.0)),
                        Expr::Literal(Literal::Number(2.0)),
                    ],
                }),
            ],
        };

        for line in ast.statements {
            execute(&line, &env).expect("execute() should return Ok(())");
        }

        // assert_eq!(res, LoxValue::Nil);
    }

    #[test]
    fn it_calls_builtin_functions() {
        let env = Environment::new();

        env.define(
            "click",
            LoxValue::Callable(InnerCallable {
                declaration: Stmt::Function {
                    name: "click".to_string(),
                    parameters: vec!["neat".to_string()],
                    body: Stmt::Block(vec![]).into(),
                },
                arity: 1,
            }),
        );

        let res = evaluate(
            &Expr::Call {
                callee: Expr::Variable("click".to_string()).into(),
                arguments: vec![Expr::Literal(Literal::True)],
            },
            &env,
        )
        .expect("eval should return Ok(())");

        // TODO: only returns nil by default, I think there's supposed to be somethig here
        assert_eq!(res, LoxValue::Nil);
    }

    #[test]
    fn it_fails_to_read_missing_variables() {
        assert_contains(
            &fail_eval(&Expr::Variable("name".to_string())),
            "\"name\" not defined",
        );
    }

    #[test]
    fn it_fails_to_evaluate_invalid_unary_expressions() {
        let err = fail_eval(&Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::String("bad".to_string())).into(),
        });
        assert_contains(&err, "invalid unary");
        assert_contains(&err, "-\"bad\"");

        let err = fail_eval(&Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::False).into(),
        });
        assert_contains(&err, "-false");

        let err = fail_eval(&Expr::Unary {
            op: UnaryOp::Neg,
            expr: Expr::Literal(Literal::Nil).into(),
        });
        assert_contains(&err, "-nil");
    }

    #[test]
    fn it_fails_to_evaluate_invalid_binary_expressions() {
        // str - num
        let err = fail_eval(&Expr::Binary {
            left: Expr::Literal(Literal::String("bad".to_string())).into(),
            op: BinaryOp::Add,
            right: Expr::Literal(Literal::Number(123.0)).into(),
        });

        assert_contains(&err, "same type");
        assert_contains(&err, "123");
        assert_contains(&err, "bad");

        // str - str
        let err = fail_eval(&Expr::Binary {
            left: Expr::Literal(Literal::String("bad".to_string())).into(),
            op: BinaryOp::Sub,
            right: Expr::Literal(Literal::String("worse".to_string())).into(),
        });
        assert_contains(&err, "\"-\" not supported between");
        assert_contains(&err, "\"bad\"");
        assert_contains(&err, "\"worse\"");
    }

    #[test]
    fn it_fails_to_call_non_callables() {
        let err = fail_eval(&Expr::Call {
            callee: Expr::Literal(Literal::String("cool".to_string())).into(),
            arguments: vec![],
        });
        assert_contains(&err, "is not callable");
    }

    #[test]
    fn it_fails_to_call_with_wrong_arity() {
        let env = Environment::new();
        env.define(
            "clock",
            LoxValue::Callable(InnerCallable {
                declaration: Stmt::Function {
                    name: "clock".to_string(),
                    parameters: vec!["neat".to_string()],
                    body: Stmt::Block(vec![]).into(),
                },
                arity: 1,
            }),
        );

        let err = evaluate(
            &Expr::Call {
                callee: Expr::Variable("clock".to_string()).into(),
                arguments: vec![],
            },
            &env,
        )
        .expect_err("eval should return Err(...)");

        assert_contains(&err, "Expected 1 arg(s)");
    }
}
