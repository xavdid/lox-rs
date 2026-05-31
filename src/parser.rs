use anyhow::{Result, anyhow};

use crate::ast::{Ast, Expr, FnDefn, Literal, Stmt};
use crate::join_errors;
use crate::scanner::{Token, TokenType, Tokens};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

type ParserResult<T = Expr> = Result<T>;

// > basic expression grammar:
// each level represents a new precedence
// the bottom of the list has the higest coupling

// expression     -> equality ;
// equality       -> comparison ( ( "!=" | "==" ) comparison )* ;
// comparison     -> term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term           -> factor ( ( "-" | "+" ) factor )* ;
// factor         -> unary ( ( "/" | "*" ) unary )* ;
// unary          -> ( "!" | "-" ) unary
//                | primary ;
// primary        -> NUMBER | STRING | "true" | "false" | "nil"
//                | "(" expression ")" ;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(mut self) -> Result<Ast, Vec<anyhow::Error>> {
        let mut statements = vec![];
        let mut errors = vec![];

        while !self.is_at_end() {
            match self.parse_declaration() {
                Ok(statement) => statements.push(statement),
                Err(err) => {
                    errors.push(err);
                    self.syncronize();
                }
            }
        }

        if errors.is_empty() {
            Ok(Ast { statements })
        } else {
            Err(errors)
        }
    }

    // this is a _recursive descent_ parser, which means it recurses all the way down until it errors (by not finding an experssion at all, or hits some other parsing issue)
    // each function represents a slightly weaker operator precedence

    fn parse_declaration(&mut self) -> ParserResult<Stmt> {
        if self.next_if(TokenType::Fun) {
            self.parse_function_declaration()
        } else if self.next_if(TokenType::Var) {
            self.parse_var_declaration()
        } else {
            self.parse_statement()
        }
    }

    // TODO: kind is probably an enum
    fn parse_function_declaration(&mut self /*, kind: String */) -> ParserResult<Stmt> {
        let kind = "function";
        let name = self.next_if_identifier(&format!("Expected {kind} name"))?;

        self.next_if_or_err(
            TokenType::LeftParen,
            &format!("Expected '(' after {kind} name"),
        )?;

        // params
        let mut parameters = vec![];
        if !self.next_is(TokenType::RightParen) {
            loop {
                if parameters.len() >= 255 {
                    return Err(self.build_error("Can't have more than 255 parameters."));
                }

                parameters.push(self.next_if_identifier("Expected parameter name")?);

                if !self.next_if(TokenType::Comma) {
                    break;
                }
            }
        }
        self.next_if_or_err(TokenType::RightParen, "Expected ')' after parameters")?;

        // body
        self.next_if_or_err(
            TokenType::LeftBrace,
            &format!("Expected '{{' before {kind} body"),
        )?;

        let body = match self.parse_block()? {
            Stmt::Block(stmts) => stmts,
            r => panic!("parse_block should always return a Stmt::Block; got {r:?}"),
        };

        Ok(Stmt::Function(FnDefn {
            name,
            parameters,
            body,
        }))
    }

    fn parse_var_declaration(&mut self) -> ParserResult<Stmt> {
        let name = self.next_if_identifier("Missing variable name.")?;

        let val = if self.next_if(TokenType::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.next_if_or_err(TokenType::Semicolon, "Expected ';' after var declaration.")?;

        Ok(Stmt::Var { name, val })
    }

    fn parse_statement(&mut self) -> ParserResult<Stmt> {
        if self.next_if(TokenType::For) {
            self.parse_for_statement()
        } else if self.next_if(TokenType::If) {
            self.parse_if_statement()
        } else if self.next_if(TokenType::Print) {
            self.parse_print_statement()
        } else if self.next_if(TokenType::While) {
            self.print_while_statement()
        } else if self.next_if(TokenType::LeftBrace) {
            self.parse_block()
        } else {
            self.parse_expression_statement()
        }
    }

    fn parse_for_statement(&mut self) -> ParserResult<Stmt> {
        self.next_if_or_err(TokenType::LeftParen, "Expected '(' after 'for'.")?;

        let initializer: Option<Stmt> = if self.next_if(TokenType::Semicolon) {
            None
        } else if self.next_if(TokenType::Var) {
            Some(self.parse_var_declaration()?)
        } else {
            Some(self.parse_expression_statement()?)
        };

        // we parse an expression if present
        let condition = if self.next_is(TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        // but always must end this section with a semicolon
        self.next_if_or_err(TokenType::Semicolon, "Expected ';' after loop condition.")?;

        // we parse an expression if present
        let increment = if self.next_is(TokenType::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        // but always must end this section with a right paren
        self.next_if_or_err(TokenType::RightParen, "Expected ')' after 'for' clauses.")?;

        let mut body = self.parse_statement()?;

        // now we turn the bits of the for loop into code around the original body.

        // for (var i = 0; i < 10; i = i + 1) {
        //      ^A         ^B      ^C
        //   print i;
        //   ^D
        // }

        // becomes:

        // {
        //   var i = 0; // A
        //   while (i < 10) { // B
        //     print i; // D
        //     i = i + 1; // C
        //   }
        // }

        // which are equivalent

        // C: if there's an increment, put it after the original body
        if let Some(increment) = increment {
            body = Stmt::Block(vec![body, Stmt::Expression(increment)]);
        };

        // B
        body = Stmt::While {
            condition: match condition {
                Some(c) => c,
                // if there's no condition, then it's a `while(true)`
                None => Expr::Literal(Literal::True),
            },
            body: body.into(),
        };

        // A
        if let Some(initializer) = initializer {
            body = Stmt::Block(vec![initializer, body])
        }

        Ok(body)
    }

    fn parse_if_statement(&mut self) -> ParserResult<Stmt> {
        self.next_if_or_err(TokenType::LeftParen, "Expected '(' after 'if'.")?;
        let condition = self.parse_expression()?;
        self.next_if_or_err(TokenType::RightParen, "Expected ')' after if condition.")?;

        let then_branch = self.parse_statement()?.into();
        let else_branch = if self.next_if(TokenType::Else) {
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_print_statement(&mut self) -> ParserResult<Stmt> {
        let value = self.parse_expression()?;
        self.next_if_or_err(TokenType::Semicolon, "Expected ';' after statement.")?;
        Ok(Stmt::Print(value))
    }

    fn print_while_statement(&mut self) -> ParserResult<Stmt> {
        self.next_if_or_err(TokenType::LeftParen, "Expected '(' after 'while'.")?;
        let condition = self.parse_expression()?;
        self.next_if_or_err(TokenType::RightParen, "Expected ')' after condition.")?;

        let body = self.parse_statement()?.into();

        Ok(Stmt::While { condition, body })
    }

    fn parse_block(&mut self) -> ParserResult<Stmt> {
        let mut res = vec![];

        while !self.next_is(TokenType::RightBrace) && !self.is_at_end() {
            res.push(self.parse_declaration()?);
        }

        self.next_if_or_err(TokenType::RightBrace, "Expected '}' after block.")?;

        Ok(Stmt::Block(res))
    }

    fn parse_expression_statement(&mut self) -> ParserResult<Stmt> {
        let value = self.parse_expression()?;
        self.next_if_or_err(TokenType::Semicolon, "Expected ';' after statement.")?;

        Ok(Stmt::Expression(value))
    }

    fn parse_expression(&mut self) -> ParserResult {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ParserResult {
        let expr = self.parse_logical_or()?;

        if self.next_if(TokenType::Equal) {
            let value = self.parse_assignment()?;

            return match &expr {
                Expr::Variable(name) => Ok(Expr::Assign {
                    name: name.to_string(),
                    value: value.into(),
                }),
                _ => Err(self.build_error("Invalid assignment target.")),
            };
        }

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> ParserResult {
        let mut expr = self.parse_logical_and()?;
        while self.next_if(TokenType::Or) {
            let op = self.previous().clone().into();
            let right = self.parse_logical_and()?.into();
            expr = Expr::Logical {
                left: expr.into(),
                op,
                right,
            }
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> ParserResult {
        let mut expr = self.parse_equality()?;

        while self.next_if(TokenType::And) {
            let op = self.previous().clone().into();
            let right = self.parse_equality()?.into();
            expr = Expr::Logical {
                left: expr.into(),
                op,
                right,
            }
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> ParserResult {
        let mut expr = self.parse_comparison()?;

        while self.next_if(TokenType::BangEqual) || self.next_if(TokenType::EqualEqual) {
            let op = self.previous().value.clone().into();
            let right = self.parse_comparison()?.into();
            expr = Expr::Binary {
                left: expr.into(),
                op,
                right,
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> ParserResult {
        let mut expr = self.parse_term()?;

        while self.next_if(TokenType::GreaterThan)
            || self.next_if(TokenType::GreaterThanEqual)
            || self.next_if(TokenType::LessThan)
            || self.next_if(TokenType::LessThanEqual)
        {
            let op = self.previous().value.clone().into();
            let right = self.parse_term()?.into();
            expr = Expr::Binary {
                left: expr.into(),
                op,
                right,
            };
        }

        Ok(expr)
    }

    // addition / subtraction
    fn parse_term(&mut self) -> ParserResult {
        let mut expr = self.parse_factor()?;

        while self.next_if(TokenType::Minus) || self.next_if(TokenType::Plus) {
            let op = self.previous().value.clone().into();
            let right = self.parse_unary()?.into();
            expr = Expr::Binary {
                left: expr.into(),
                op,
                right,
            };
        }

        Ok(expr)
    }

    // multiplication / division
    fn parse_factor(&mut self) -> ParserResult {
        let mut expr = self.parse_unary()?;

        while self.next_if(TokenType::Slash) || self.next_if(TokenType::Star) {
            let op = self.previous().value.clone().into();
            let right = self.parse_unary()?.into();
            expr = Expr::Binary {
                left: expr.into(),
                op,
                right,
            };
        }

        Ok(expr)
    }

    // infix operators, like `!true` and `-1`
    fn parse_unary(&mut self) -> ParserResult {
        if self.next_if(TokenType::Bang) || self.next_if(TokenType::Minus) {
            let op = self.previous().value.clone().into();
            let expr = self.parse_unary()?.into();
            Ok(Expr::Unary { op, expr })
        } else {
            self.parse_call()
        }
    }

    /// invoking anything that's callable
    fn parse_call(&mut self) -> ParserResult {
        let mut expr = self.parse_primary()?;

        while self.next_if(TokenType::LeftParen) {
            // inlined `finishCall` from book
            expr = self.finish_call(expr)?;
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> ParserResult {
        let mut arguments = vec![];

        if !self.next_is(TokenType::RightParen) {
            loop {
                arguments.push(self.parse_expression()?);

                if !self.next_if(TokenType::Comma) {
                    break;
                }
            }
        }

        self.next_if_or_err(TokenType::RightParen, "Expected ')' after arguments")?;

        // here, the book warns for functions with more than 255 arguments. Not worth it though.

        Ok(Expr::Call {
            callee: callee.into(),
            arguments,
        })
    }

    // this is our eventual base case (with the highest precedence)- literals no longer recurse
    fn parse_primary(&mut self) -> ParserResult {
        if let Some(literal) = match &self.peek() {
            TokenType::Nil => Some(Literal::Nil),
            TokenType::True => Some(Literal::True),
            TokenType::False => Some(Literal::False),
            TokenType::String(s) => Some(Literal::String(s.to_owned())),
            TokenType::Number(s) => Some(Literal::Number(
                s.parse()
                    .expect("expected {s} to be a valid f64; scanner has a bug"),
            )),
            _ => None,
        } {
            self.next();
            return Ok(Expr::Literal(literal));
        }

        if let TokenType::Identifier(name) = &self.peek() {
            let name = name.clone(); // escape the borrow from self
            self.next();
            return Ok(Expr::Variable(name));
        }

        // otherwise, try a grouping
        if self.next_if(TokenType::LeftParen) {
            let expr = self.parse_expression()?;

            self.next_if_or_err(TokenType::RightParen, "Missing ')' after expression.")?;

            return Ok(Expr::Grouping(expr.into()));
        }

        Err(anyhow!(
            "(bottom of table) Expected expression, got {:?}",
            self.peek()
        ))
    }

    // HELPERS

    fn next_if_or_err(&mut self, t: TokenType, msg: &str) -> ParserResult<()> {
        if self.next_if(t) {
            Ok(())
        } else {
            Err(self.build_error(msg))
        }
    }

    fn build_error(&self, msg: &str) -> anyhow::Error {
        anyhow!("[{:?}] {msg}", self.peek())
    }

    /// book calls this `match`, but I don't like that it doesn't communicate that it advances the pointer. Returns whether it matched and advanced
    // TODO: option? we always call .previous() right after
    fn next_if(&mut self, token_type: TokenType) -> bool {
        if self.peek() == &token_type {
            self.next();
            true
        } else {
            false
        }
    }
    /// this like `next_if` but hardcodes Identifier since I can't match my enums that hold values as a function arg
    fn next_if_identifier(&mut self, err_msg: &str) -> ParserResult<String> {
        if matches!(self.peek(), TokenType::Identifier(_)) {
            if let TokenType::Identifier(name) = &self.next().value {
                Ok(name.to_string())
            } else {
                panic!(".peek() said we had an idenitifier, but we didn't?")
            }
        } else {
            Err(self.build_error(err_msg))
        }
    }

    fn is_at_end(&self) -> bool {
        self.next_is(TokenType::Eof)
    }

    fn peek(&self) -> &TokenType {
        &self.tokens[self.current].value
    }

    fn next(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn next_is(&self, t: TokenType) -> bool {
        self.peek() == &t
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /// if we hit a parser error, call this to fast forward until we hit what we think is the start of a statement. This minimizes cascading errors
    fn syncronize(&mut self) {
        use TokenType::*;

        self.next();

        while !self.is_at_end() {
            if self.previous().value == Semicolon {
                return;
            }

            if matches!(
                self.peek(),
                Class | Fun | Var | For | If | While | Print | Return
            ) {
                return;
            }

            self.next();
        }
    }
}

pub fn parse(tokens: Tokens) -> Result<Ast> {
    Parser::new(tokens.tokens).parse().map_err(join_errors)
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::ast::*;
    use crate::test_util::assert_contains;
    use anyhow::Error;
    use pretty_assertions::assert_eq;

    /// helper for more legible token-heavy tests
    fn token(value: TokenType) -> Token {
        Token { value, line: 1 }
    }

    fn parse(tokens: Vec<Token>) -> Ast {
        Parser::new(tokens)
            .parse()
            .expect("this method should return Ok(), got Err()")
    }
    fn fail_parse(tokens: Vec<Token>) -> Vec<Error> {
        Parser::new(tokens)
            .parse()
            .expect_err("this method should return Err()")
    }

    #[test]
    fn it_parses_basic_nil() {
        let result = parse(vec![
            token(TokenType::Nil),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Nil))]
            }
        )
    }

    #[test]
    fn it_parses_basic_bool() {
        let result = parse(vec![
            token(TokenType::False),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::False))]
            }
        )
    }

    #[test]
    fn it_parses_basic_string() {
        let result = parse(vec![
            token(TokenType::String("cool".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::String(
                    "cool".to_string()
                )))]
            }
        )
    }

    #[test]
    fn it_parses_basic_int() {
        let result = parse(vec![
            token(TokenType::Number("123".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Number(123.0)))]
            }
        )
    }

    #[test]
    fn it_parses_basic_float() {
        let result = parse(vec![
            token(TokenType::Number("123.456".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Number(123.456)))]
            }
        )
    }

    #[test]
    fn it_parses_basic_grouping() {
        let result = parse(vec![
            token(TokenType::LeftParen),
            token(TokenType::True),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Grouping(
                    Expr::Literal(Literal::True).into()
                ))]
            }
        )
    }

    #[test]
    fn it_parses_a_function_call_without_args() {
        let result = parse(vec![
            token(TokenType::Identifier("cool".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Call {
                    callee: Expr::Variable("cool".to_string()).into(),
                    arguments: vec![]
                })]
            }
        )
    }

    #[test]
    fn it_parses_a_function_call_with_2_args() {
        let result = parse(vec![
            token(TokenType::Identifier("add".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Number("1".to_string())),
            token(TokenType::Comma),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Call {
                    callee: Expr::Variable("add".to_string()).into(),
                    arguments: vec![
                        Expr::Literal(Literal::Number(1.0)),
                        Expr::Literal(Literal::Number(2.0))
                    ]
                })]
            }
        )
    }

    #[test]
    fn it_parses_chained_function_calls() {
        let result = parse(vec![
            token(TokenType::Identifier("adder".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Number("1".to_string())),
            token(TokenType::Comma),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftParen),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Call {
                    callee: Expr::Call {
                        callee: Expr::Variable("adder".to_string()).into(),
                        arguments: vec![
                            Expr::Literal(Literal::Number(1.0)),
                            Expr::Literal(Literal::Number(2.0))
                        ]
                    }
                    .into(),
                    arguments: vec![]
                })]
            }
        )
    }

    #[test]
    fn it_parses_nested_function_calls() {
        let result = parse(vec![
            token(TokenType::Identifier("f".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Identifier("g".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::RightParen),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Call {
                    callee: Expr::Variable("f".to_string()).into(),
                    arguments: vec![Expr::Call {
                        callee: Expr::Variable("g".to_string()).into(),
                        arguments: vec![]
                    }]
                })]
            }
        )
    }

    #[test]
    fn it_parses_basic_function_declarations() {
        let result = parse(vec![
            token(TokenType::Fun),
            token(TokenType::Identifier("f".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Function(FnDefn {
                    name: "f".into(),
                    parameters: vec![],
                    body: vec![]
                })]
            }
        )
    }

    #[test]
    fn it_parses_function_declaration_with_args_and_body() {
        let result = parse(vec![
            token(TokenType::Fun),
            token(TokenType::Identifier("sum".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Comma),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Plus),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Identifier("sum".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Number("1".to_string())),
            token(TokenType::Comma),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![
                    Stmt::Function(FnDefn {
                        name: "sum".into(),
                        parameters: vec!["a".to_string(), "b".to_string()],
                        body: vec![Stmt::Print(Expr::Binary {
                            left: Expr::Variable("a".to_string()).into(),
                            op: BinaryOp::Add,
                            right: Expr::Variable("b".to_string()).into(),
                        })]
                    }),
                    Stmt::Expression(Expr::Call {
                        callee: Expr::Variable("sum".to_string()).into(),
                        arguments: vec![
                            Expr::Literal(Literal::Number(1.0)),
                            Expr::Literal(Literal::Number(2.0))
                        ],
                    })
                ]
            }
        )
    }

    #[test]
    fn it_parses_basic_unary() {
        let result = parse(vec![
            token(TokenType::Minus),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Literal(Literal::Number(3.0)).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_basic_binary() {
        let result = parse(vec![
            token(TokenType::Number("1".to_string())),
            token(TokenType::Slash),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Binary {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: BinaryOp::Div,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_flat_nested_binary() {
        let result = parse(vec![
            token(TokenType::Number("1".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Minus),
            token(TokenType::Number("4".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Binary {
                    left: Expr::Binary {
                        left: Expr::Binary {
                            left: Expr::Literal(Literal::Number(1.0)).into(),
                            op: BinaryOp::Add,
                            right: Expr::Literal(Literal::Number(2.0)).into(),
                        }
                        .into(),
                        op: BinaryOp::Add,
                        right: Expr::Literal(Literal::Number(3.0)).into(),
                    }
                    .into(),
                    op: BinaryOp::Sub,
                    right: Expr::Literal(Literal::Number(4.0)).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_nested_unary() {
        let result = parse(vec![
            token(TokenType::Bang),
            token(TokenType::Bang),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Expr::Unary {
                        op: UnaryOp::Not,
                        expr: Expr::Literal(Literal::Number(2.0)).into()
                    }
                    .into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_nested_binary() {
        let result = parse(vec![
            token(TokenType::Number("3".to_string())),
            token(TokenType::Star),
            token(TokenType::LeftParen),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Slash),
            token(TokenType::Number("4".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Binary {
                    left: Expr::Literal(Literal::Number(3.0)).into(),
                    op: BinaryOp::Mul,
                    right: Expr::Grouping(
                        Expr::Binary {
                            left: Expr::Literal(Literal::Number(2.0)).into(),
                            op: BinaryOp::Div,
                            right: Expr::Literal(Literal::Number(4.0)).into()
                        }
                        .into()
                    )
                    .into()
                })]
            }
        )
    }

    #[test]
    fn it_mixes_unary_and_binary() {
        let result = parse(vec![
            token(TokenType::Minus),
            token(TokenType::Number("3".to_string())),
            token(TokenType::LessThanEqual),
            token(TokenType::LeftParen),
            token(TokenType::String("neat".to_string())),
            token(TokenType::EqualEqual),
            token(TokenType::Bang),
            token(TokenType::Nil),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Binary {
                    left: Expr::Unary {
                        op: UnaryOp::Neg,
                        expr: Expr::Literal(Literal::Number(3.0)).into()
                    }
                    .into(),
                    op: BinaryOp::Lte,
                    right: Expr::Grouping(
                        Expr::Binary {
                            left: Expr::Literal(Literal::String("neat".to_string())).into(),
                            op: BinaryOp::Eq,
                            right: Expr::Unary {
                                op: UnaryOp::Not,
                                expr: Expr::Literal(Literal::Nil).into()
                            }
                            .into()
                        }
                        .into()
                    )
                    .into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_print_statements() {
        let result = parse(vec![
            token(TokenType::Print),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Print(Expr::Literal(Literal::Number(3.0)))]
            }
        )
    }

    #[test]
    fn it_parses_block_statements() {
        let result = parse(vec![
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Print),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Block(vec![
                    Stmt::Print(Expr::Literal(Literal::Number(2.0))),
                    Stmt::Print(Expr::Literal(Literal::Number(3.0)))
                ])]
            }
        )
    }

    #[test]
    fn it_parses_an_empty_block() {
        let result = parse(vec![
            token(TokenType::LeftBrace),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Block(vec![])]
            }
        )
    }

    #[test]
    fn it_parses_if_statements_no_else() {
        let result = parse(vec![
            token(TokenType::If),
            token(TokenType::LeftParen),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            Token {
                value: TokenType::Print,
                line: 2,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 2,
            },
            Token {
                value: TokenType::Semicolon,
                line: 2,
            },
            Token {
                value: TokenType::RightBrace,
                line: 3,
            },
            Token {
                value: TokenType::Eof,
                line: 3,
            },
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::If {
                    condition: Expr::Literal(Literal::Number(2.0)),
                    then_branch: Stmt::Block(vec![Stmt::Print(Expr::Literal(Literal::Number(
                        3.0
                    )))])
                    .into(),
                    else_branch: None
                }]
            }
        )
    }

    #[test]
    fn it_parses_if_statements_else() {
        let result = parse(vec![
            token(TokenType::If),
            token(TokenType::LeftParen),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            Token {
                value: TokenType::Print,
                line: 2,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 2,
            },
            Token {
                value: TokenType::Semicolon,
                line: 2,
            },
            Token {
                value: TokenType::RightBrace,
                line: 3,
            },
            Token {
                value: TokenType::Else,
                line: 3,
            },
            Token {
                value: TokenType::LeftBrace,
                line: 3,
            },
            Token {
                value: TokenType::Print,
                line: 4,
            },
            Token {
                value: TokenType::Number("-1".to_string()),
                line: 4,
            },
            Token {
                value: TokenType::Semicolon,
                line: 4,
            },
            Token {
                value: TokenType::RightBrace,
                line: 5,
            },
            Token {
                value: TokenType::Eof,
                line: 5,
            },
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::If {
                    condition: Expr::Literal(Literal::Number(2.0)),
                    then_branch: Stmt::Block(vec![Stmt::Print(Expr::Literal(Literal::Number(
                        3.0
                    )))])
                    .into(),
                    else_branch: Some(
                        Stmt::Block(vec![Stmt::Print(Expr::Literal(Literal::Number(-1.0)))]).into()
                    )
                }]
            }
        )
    }

    #[test]
    fn it_fails_on_bad_block_statements() {
        let errors = fail_parse(vec![
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Number("2".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Print),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "expected '}'");
    }

    #[test]
    fn it_parses_while_statements() {
        let result = parse(vec![
            token(TokenType::While),
            token(TokenType::LeftParen),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            Token {
                value: TokenType::Print,
                line: 2,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 2,
            },
            Token {
                value: TokenType::Semicolon,
                line: 2,
            },
            Token {
                value: TokenType::RightBrace,
                line: 3,
            },
            Token {
                value: TokenType::Eof,
                line: 3,
            },
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::While {
                    condition: Expr::Literal(Literal::Number(2.0)),
                    body: Stmt::Block(vec![Stmt::Print(Expr::Literal(Literal::Number(3.0)))])
                        .into(),
                }]
            }
        )
    }

    #[test]
    fn it_fails_bad_while_statements() {
        let errors = fail_parse(vec![
            token(TokenType::While),
            token(TokenType::LeftParen),
            token(TokenType::Number("2".to_string())),
            token(TokenType::LeftBrace),
            Token {
                value: TokenType::Print,
                line: 2,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 2,
            },
            Token {
                value: TokenType::Semicolon,
                line: 2,
            },
            Token {
                value: TokenType::RightBrace,
                line: 3,
            },
            Token {
                value: TokenType::Eof,
                line: 3,
            },
        ]);

        // assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "expected ')'");
    }

    #[test]
    fn it_parses_full_for_statements() {
        let result = parse(vec![
            token(TokenType::For),
            token(TokenType::LeftParen),
            token(TokenType::Var),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Number("0".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::LessThan),
            token(TokenType::Number("5".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("1".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                // {
                //   var i = 0; // A
                //   while (i < 10) { // B
                //     print i; // D
                //     i = i + 1; // C
                //   }
                // }
                statements: vec![Stmt::Block(vec![
                    Stmt::Var {
                        name: "i".to_string(),
                        val: Some(Expr::Literal(Literal::Number(0.0)))
                    },
                    Stmt::While {
                        condition: Expr::Binary {
                            left: Expr::Variable("i".to_string()).into(),
                            op: BinaryOp::Lt,
                            right: Expr::Literal(Literal::Number(5.0)).into()
                        },
                        body: Stmt::Block(vec![
                            Stmt::Block(vec![Stmt::Print(Expr::Variable("i".to_string()))]),
                            Stmt::Expression(Expr::Assign {
                                name: "i".to_string(),
                                value: Expr::Binary {
                                    left: Expr::Variable("i".to_string()).into(),
                                    op: BinaryOp::Add,
                                    right: Expr::Literal(Literal::Number(1.0)).into()
                                }
                                .into()
                            })
                        ])
                        .into(),
                    }
                ])]
            }
        )
    }

    #[test]
    fn it_parses_for_statements_no_condition() {
        let result = parse(vec![
            token(TokenType::For),
            token(TokenType::LeftParen),
            token(TokenType::Var),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Number("0".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("1".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Block(vec![
                    Stmt::Var {
                        name: "i".to_string(),
                        val: Some(Expr::Literal(Literal::Number(0.0)))
                    },
                    Stmt::While {
                        condition: Expr::Literal(Literal::True),
                        body: Stmt::Block(vec![
                            Stmt::Block(vec![Stmt::Print(Expr::Variable("i".to_string()))]),
                            Stmt::Expression(Expr::Assign {
                                name: "i".to_string(),
                                value: Expr::Binary {
                                    left: Expr::Variable("i".to_string()).into(),
                                    op: BinaryOp::Add,
                                    right: Expr::Literal(Literal::Number(1.0)).into()
                                }
                                .into()
                            })
                        ])
                        .into(),
                    }
                ])]
            }
        )
    }

    #[test]
    fn it_parses_for_statements_no_increment() {
        let result = parse(vec![
            token(TokenType::For),
            token(TokenType::LeftParen),
            token(TokenType::Var),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Number("0".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::LessThan),
            token(TokenType::Number("5".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Block(vec![
                    Stmt::Var {
                        name: "i".to_string(),
                        val: Some(Expr::Literal(Literal::Number(0.0)))
                    },
                    Stmt::While {
                        condition: Expr::Binary {
                            left: Expr::Variable("i".to_string()).into(),
                            op: BinaryOp::Lt,
                            right: Expr::Literal(Literal::Number(5.0)).into()
                        },
                        body: Stmt::Block(vec![Stmt::Print(Expr::Variable("i".to_string()))])
                            .into(),
                    }
                ])]
            }
        )
    }

    #[test]
    fn it_parses_for_statement_no_initializer() {
        let result = parse(vec![
            token(TokenType::For),
            token(TokenType::LeftParen),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::LessThan),
            token(TokenType::Number("5".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Equal),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("1".to_string())),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                // {
                //   var i = 0; // A
                //   while (i < 10) { // B
                //     print i; // D
                //     i = i + 1; // C
                //   }
                // }
                statements: vec![Stmt::While {
                    condition: Expr::Binary {
                        left: Expr::Variable("i".to_string()).into(),
                        op: BinaryOp::Lt,
                        right: Expr::Literal(Literal::Number(5.0)).into()
                    },
                    body: Stmt::Block(vec![
                        Stmt::Block(vec![Stmt::Print(Expr::Variable("i".to_string()))]),
                        Stmt::Expression(Expr::Assign {
                            name: "i".to_string(),
                            value: Expr::Binary {
                                left: Expr::Variable("i".to_string()).into(),
                                op: BinaryOp::Add,
                                right: Expr::Literal(Literal::Number(1.0)).into()
                            }
                            .into()
                        })
                    ])
                    .into(),
                }]
            }
        )
    }

    #[test]
    fn it_parses_minimal_for_statements() {
        let result = parse(vec![
            token(TokenType::For),
            token(TokenType::LeftParen),
            token(TokenType::Semicolon),
            token(TokenType::Semicolon),
            token(TokenType::RightParen),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("i".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                // {
                //   var i = 0; // A
                //   while (i < 10) { // B
                //     print i; // D
                //     i = i + 1; // C
                //   }
                // }
                statements: vec![Stmt::While {
                    condition: Expr::Literal(Literal::True),
                    body: Stmt::Block(vec![Stmt::Print(Expr::Variable("i".to_string()))]).into(),
                }]
            }
        )
    }

    #[test]
    fn it_parses_var_declarations_with_values() {
        let result = parse(vec![
            token(TokenType::Var),
            token(TokenType::Identifier("name".to_string())),
            token(TokenType::Equal),
            token(TokenType::String("david".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Var {
                    name: "name".to_string(),
                    val: Some(Expr::Literal(Literal::String("david".to_string())))
                }]
            }
        )
    }

    #[test]
    fn it_parses_var_declarations_with_complex_values() {
        let result = parse(vec![
            token(TokenType::Var),
            token(TokenType::Identifier("name".to_string())),
            token(TokenType::Equal),
            token(TokenType::LeftParen),
            token(TokenType::Number("1".to_string())),
            token(TokenType::Plus),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Star),
            token(TokenType::Number("3".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Var {
                    name: "name".to_string(),
                    val: Some(Expr::Binary {
                        left: Expr::Grouping(
                            Expr::Binary {
                                left: Expr::Literal(Literal::Number(1.0)).into(),
                                op: BinaryOp::Add,
                                right: Expr::Literal(Literal::Number(2.0)).into()
                            }
                            .into()
                        )
                        .into(),
                        op: BinaryOp::Mul,
                        right: Expr::Literal(Literal::Number(3.0)).into()
                    })
                }]
            }
        )
    }

    #[test]
    fn it_parses_empty_var_declarations() {
        let result = parse(vec![
            token(TokenType::Var),
            token(TokenType::Identifier("name".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Var {
                    name: "name".to_string(),
                    val: None
                }]
            }
        )
    }

    #[test]
    fn it_parses_var_access() {
        let result = parse(vec![
            token(TokenType::Identifier("name".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Variable("name".to_string()))]
            }
        )
    }

    #[test]
    fn it_parses_nested_var_access() {
        let result = parse(vec![
            token(TokenType::Minus),
            token(TokenType::Identifier("name".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Variable("name".to_string()).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_logical_or() {
        let result = parse(vec![
            token(TokenType::String("name".to_string())),
            token(TokenType::Or),
            token(TokenType::String("age".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Logical {
                    left: Expr::Literal(Literal::String("name".to_string())).into(),
                    op: LogicalOp::Or,
                    right: Expr::Literal(Literal::String("age".to_string())).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_logical_and() {
        let result = parse(vec![
            token(TokenType::Number("123".to_string())),
            token(TokenType::And),
            token(TokenType::Number("456".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Logical {
                    left: Expr::Literal(Literal::Number(123.0)).into(),
                    op: LogicalOp::And,
                    right: Expr::Literal(Literal::Number(456.0)).into()
                })]
            }
        )
    }

    #[test]
    fn it_parses_nested_logical_operators() {
        let result = parse(vec![
            token(TokenType::Number("123".to_string())),
            token(TokenType::And),
            token(TokenType::Number("456".to_string())),
            token(TokenType::And),
            token(TokenType::Number("789".to_string())),
            token(TokenType::Or),
            token(TokenType::Number("890".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(
            result,
            Ast {
                statements: vec![Stmt::Expression(Expr::Logical {
                    left: Expr::Logical {
                        left: Expr::Logical {
                            left: Expr::Literal(Literal::Number(123.0)).into(),
                            op: LogicalOp::And,
                            right: Expr::Literal(Literal::Number(456.0)).into()
                        }
                        .into(),
                        op: LogicalOp::And,
                        right: Expr::Literal(Literal::Number(789.0)).into()
                    }
                    .into(),
                    op: LogicalOp::Or,
                    right: Expr::Literal(Literal::Number(890.0)).into()
                })]
            }
        )
    }

    #[test]
    fn it_fails_for_broken_grouping() {
        let errors = fail_parse(vec![
            token(TokenType::LeftParen),
            token(TokenType::True),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "missing ')'");
    }

    #[test]
    fn it_fails_for_missing_semi_in_print() {
        let errors = fail_parse(vec![
            token(TokenType::Print),
            token(TokenType::True),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "expected ';'");
    }

    #[test]
    fn it_fails_for_invalid_assignment() {
        let errors = fail_parse(vec![
            token(TokenType::Number("3".to_string())),
            token(TokenType::Equal),
            token(TokenType::Number("5".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "invalid assignment target");
    }

    #[test]
    fn it_fails_for_invalid_function_call() {
        let errors = fail_parse(vec![
            token(TokenType::Identifier("add".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Number("1".to_string())),
            token(TokenType::Number("2".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "expected ')'");
    }

    #[test]
    fn it_fails_for_invalid_function_declaration_no_rparen() {
        let errors = fail_parse(vec![
            token(TokenType::Fun),
            token(TokenType::Identifier("sum".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Comma),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::LeftBrace),
            token(TokenType::Print),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Plus),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 2);
        assert_contains(&errors[0], "expected ')'");
    }

    #[test]
    fn it_fails_for_invalid_function_declaration_no_lbrace() {
        let errors = fail_parse(vec![
            token(TokenType::Fun),
            token(TokenType::Identifier("sum".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Comma),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::RightParen),
            token(TokenType::Print),
            token(TokenType::Identifier("a".to_string())),
            token(TokenType::Plus),
            token(TokenType::Identifier("b".to_string())),
            token(TokenType::Semicolon),
            token(TokenType::RightBrace),
            token(TokenType::Eof),
        ]);

        assert_eq!(errors.len(), 2);
        assert_contains(&errors[0], "expected '{'");
    }

    #[test]
    fn it_doesnt_yet_handle_dangling_commas() {
        let errors = fail_parse(vec![
            token(TokenType::Identifier("f".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::Identifier("g".to_string())),
            token(TokenType::LeftParen),
            token(TokenType::RightParen),
            // TODO: it would be cool to allow trailing commas, es5 style
            token(TokenType::Comma),
            token(TokenType::RightParen),
            token(TokenType::Semicolon),
            token(TokenType::Eof),
        ]);
        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "expected expression");
    }
}
