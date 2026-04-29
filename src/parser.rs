use std::fmt::Display;

use crate::ast::{Ast, Expr, Literal, Stmt};
use crate::scanner::{Token, TokenType, Tokens};

#[derive(Debug, PartialEq)]
pub enum ParserError {
    NoExpression,
    MissingRParen { token: Token },
    MissingRBrace { token: Token },
    MissingSemiColon { token: Token },
    MissingVarName { token: Token },
    InvalidAssignmentTarget { token: Token },
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::MissingRParen { token } => {
                write!(f, "[{token}] Missing ')' after expression.")
            }
            ParserError::MissingRBrace { token } => {
                write!(f, "[{token}] Missing '}}' after block.")
            }
            ParserError::MissingSemiColon { token } => {
                write!(f, "[{token}] Missing ';' after statement.")
            }
            ParserError::MissingVarName { token } => write!(f, "[{token}] Missing variable name."),
            ParserError::NoExpression => write!(f, "No expressions at all?"),
            ParserError::InvalidAssignmentTarget { token } => {
                write!(f, "[{token}] Invaild assignment target.")
            }
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

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

    pub fn parse(mut self) -> Result<Ast, Vec<ParserError>> {
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

    fn parse_declaration(&mut self) -> Result<Stmt, ParserError> {
        if self.next_if(TokenType::Var) {
            self.parse_var_declaration()
        } else {
            self.parse_statement()
        }
    }

    fn parse_var_declaration(&mut self) -> Result<Stmt, ParserError> {
        let name = self.consume_identifier()?;

        let val = if self.next_if(TokenType::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        if self.next_if(TokenType::Semicolon) {
            Ok(Stmt::Var { name, val })
        } else {
            Err(ParserError::MissingSemiColon {
                token: (*self.peek()).clone(),
            })
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParserError> {
        if self.next_if(TokenType::If) {
            self.parse_if_statement()
        } else if self.next_if(TokenType::Print) {
            self.parse_print_statement()
        } else if self.next_if(TokenType::LeftBrace) {
            self.parse_block()
        } else {
            self.parse_expression_statement()
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, ParserError> {
        if !self.next_if(TokenType::LeftParen) {
            return Err(ParserError::MissingRParen {
                token: (*self.peek()).clone(),
            });
        }

        let condition = self.parse_expression()?;

        if !self.next_if(TokenType::RightParen) {
            return Err(ParserError::MissingRParen {
                token: (*self.peek()).clone(),
            });
        }

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

    fn parse_print_statement(&mut self) -> Result<Stmt, ParserError> {
        let value = self.parse_expression()?;
        if self.next_if(TokenType::Semicolon) {
            Ok(Stmt::Print(value))
        } else {
            Err(ParserError::MissingSemiColon {
                token: (*self.peek()).clone(),
            })
        }
    }

    fn parse_block(&mut self) -> Result<Stmt, ParserError> {
        let mut res = vec![];

        while !matches!(self.peek().value, TokenType::RightBrace) && !self.is_at_end() {
            res.push(self.parse_declaration()?);
        }

        if self.next_if(TokenType::RightBrace) {
            Ok(Stmt::Block(res))
        } else {
            Err(ParserError::MissingRParen {
                token: (*self.peek()).clone(),
            })
        }
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt, ParserError> {
        let value = self.parse_expression()?;
        if self.next_if(TokenType::Semicolon) {
            Ok(Stmt::Expression(value))
        } else {
            Err(ParserError::MissingSemiColon {
                token: (*self.peek()).clone(),
            })
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParserError> {
        let expr = self.parse_equality()?;

        if self.next_if(TokenType::Equal) {
            let equals_token = self.previous();
            let token = equals_token.clone();
            let value = self.parse_assignment()?;

            return match &expr {
                Expr::Variable(name) => Ok(Expr::Assign {
                    name: name.to_string(),
                    value: value.into(),
                }),
                _ => Err(ParserError::InvalidAssignmentTarget { token }),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParserError> {
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

    fn parse_comparison(&mut self) -> Result<Expr, ParserError> {
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
    fn parse_term(&mut self) -> Result<Expr, ParserError> {
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
    fn parse_factor(&mut self) -> Result<Expr, ParserError> {
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
    fn parse_unary(&mut self) -> Result<Expr, ParserError> {
        if self.next_if(TokenType::Bang) || self.next_if(TokenType::Minus) {
            let op = self.previous().value.clone().into();
            let expr = self.parse_unary()?.into();
            Ok(Expr::Unary { op, expr })
        } else {
            self.parse_primary()
        }
    }

    // this is our eventual base case (with the highest precedence)- literals no longer recurse
    fn parse_primary(&mut self) -> Result<Expr, ParserError> {
        if let Some(literal) = match &self.peek().value {
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

        if let TokenType::Identifier(name) = &self.peek().value {
            let name = name.clone(); // escape the borrow from self
            self.next();
            return Ok(Expr::Variable(name));
        }

        // otherwise, try a grouping
        if self.next_if(TokenType::LeftParen) {
            let expr = self.parse_expression()?;

            if self.next_if(TokenType::RightParen) {
                return Ok(Expr::Grouping(expr.into()));
            } else {
                return Err(ParserError::MissingRParen {
                    token: (*self.peek()).clone(),
                });
            }
        }

        Err(ParserError::NoExpression)
    }

    // HELPERS

    /** book calls this `match`, but I don't like that it doesn't communicate that it advances the pointer. Returns whether it matched and advanced */
    // TODO: option? we always call .previous() right after
    fn next_if(&mut self, token_type: TokenType) -> bool {
        if self.peek().value == token_type {
            self.next();
            true
        } else {
            false
        }
    }
    /**  this like `consume_if` but hardcodes Identifier since I can't match my enums that hold values as a function arg */
    fn consume_identifier(&mut self) -> Result<String, ParserError> {
        if matches!(self.peek().value, TokenType::Identifier(_)) {
            if let TokenType::Identifier(name) = &self.next().value {
                Ok(name.to_string())
            } else {
                panic!(".peek() said we had an idenitifier, but we didn't?")
            }
        } else {
            // TODO: util function- we do this a few places
            Err(ParserError::MissingVarName {
                token: (*self.peek()).clone(),
            })
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek().value == TokenType::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn next(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    /** if we hit a parser error, call this to fast forward until we hit what we think is the start of a statement. This minimizes cascading errors */
    fn syncronize(&mut self) {
        use TokenType::*;

        self.next();

        while !self.is_at_end() {
            if self.previous().value == Semicolon {
                return;
            }

            if matches!(
                self.peek().value,
                Class | Fun | Var | For | If | While | Print | Return
            ) {
                return;
            }

            self.next();
        }
    }
}

pub fn parse(tokens: Tokens) -> Result<Ast, Vec<ParserError>> {
    Parser::new(tokens.tokens).parse()
}

#[cfg(test)]
mod tests {
    use crate::ast::*;

    use super::*;

    #[test]
    fn it_parses_basic_nil() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Nil,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Nil))]
            })
        )
    }

    #[test]
    fn it_parses_basic_bool() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::False,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::False))]
            })
        )
    }

    #[test]
    fn it_parses_basic_string() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::String("cool".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::String(
                    "cool".to_string()
                )))]
            })
        )
    }

    #[test]
    fn it_parses_basic_int() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("123".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Number(123.0)))]
            })
        )
    }

    #[test]
    fn it_parses_basic_float() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("123.456".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Literal(Literal::Number(123.456)))]
            })
        )
    }

    #[test]
    fn it_parses_basic_grouping() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::True,
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Grouping(
                    Expr::Literal(Literal::True).into()
                ))]
            })
        )
    }

    #[test]
    fn it_parses_basic_unary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Minus,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Literal(Literal::Number(3.0)).into()
                })]
            })
        )
    }

    #[test]
    fn it_parses_basic_binary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("1".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Slash,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Binary {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: BinaryOp::Div,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                })]
            })
        )
    }

    #[test]
    fn it_parses_flat_nested_binary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("1".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Plus,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Plus,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Minus,
                line: 1,
            },
            Token {
                value: TokenType::Number("4".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
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
            })
        )
    }

    #[test]
    fn it_parses_nested_unary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Bang,
                line: 1,
            },
            Token {
                value: TokenType::Bang,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Expr::Unary {
                        op: UnaryOp::Not,
                        expr: Expr::Literal(Literal::Number(2.0)).into()
                    }
                    .into()
                })]
            })
        )
    }

    #[test]
    fn it_parses_nested_binary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Star,
                line: 1,
            },
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Slash,
                line: 1,
            },
            Token {
                value: TokenType::Number("4".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
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
            })
        )
    }

    #[test]
    fn it_mixes_unary_and_binary() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Minus,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::LessThanEqual,
                line: 1,
            },
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::String("neat".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::EqualEqual,
                line: 1,
            },
            Token {
                value: TokenType::Bang,
                line: 1,
            },
            Token {
                value: TokenType::Nil,
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
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
            })
        )
    }

    #[test]
    fn it_parses_print_statements() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Print(Expr::Literal(Literal::Number(3.0)))]
            })
        )
    }

    #[test]
    fn it_parses_block_statements() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::LeftBrace,
                line: 1,
            },
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::RightBrace,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Block(vec![
                    Stmt::Print(Expr::Literal(Literal::Number(2.0))),
                    Stmt::Print(Expr::Literal(Literal::Number(3.0)))
                ])]
            })
        )
    }

    #[test]
    fn it_parses_if_statements_no_else() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::If,
                line: 1,
            },
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::LeftBrace,
                line: 1,
            },
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
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::If {
                    condition: Expr::Literal(Literal::Number(2.0)),
                    then_branch: Stmt::Block(vec![Stmt::Print(Expr::Literal(Literal::Number(
                        3.0
                    )))])
                    .into(),
                    else_branch: None
                }]
            })
        )
    }

    #[test]
    fn it_parses_if_statements_else() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::If,
                line: 1,
            },
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::LeftBrace,
                line: 1,
            },
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
            parser.parse(),
            Ok(Ast {
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
            })
        )
    }

    #[test]
    fn it_fails_on_bad_block_statements() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::LeftBrace,
                line: 1,
            },
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);

        let res = parser.parse().unwrap_err();
        assert_eq!(res.len(), 1);
        assert_eq!(
            res.first().unwrap(),
            &ParserError::MissingRParen {
                token: Token {
                    value: TokenType::Eof,
                    line: 1
                }
            }
        );
    }

    #[test]
    fn it_parses_var_declarations_with_values() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Var,
                line: 1,
            },
            Token {
                value: TokenType::Identifier("name".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Equal,
                line: 1,
            },
            Token {
                value: TokenType::String("david".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Var {
                    name: "name".to_string(),
                    val: Some(Expr::Literal(Literal::String("david".to_string())))
                }]
            })
        )
    }

    #[test]
    fn it_parses_var_declarations_with_complex_values() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Var,
                line: 1,
            },
            Token {
                value: TokenType::Identifier("name".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Equal,
                line: 1,
            },
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::Number("1".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Plus,
                line: 1,
            },
            Token {
                value: TokenType::Number("2".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::RightParen,
                line: 1,
            },
            Token {
                value: TokenType::Star,
                line: 1,
            },
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
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
            })
        )
    }

    #[test]
    fn it_parses_empty_var_declarations() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Var,
                line: 1,
            },
            Token {
                value: TokenType::Identifier("name".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Var {
                    name: "name".to_string(),
                    val: None
                }]
            })
        )
    }

    #[test]
    fn it_parses_var_access() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Identifier("name".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Variable("name".to_string()))]
            })
        )
    }

    #[test]
    fn it_parses_nested_var_access() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Minus,
                line: 1,
            },
            Token {
                value: TokenType::Identifier("name".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                statements: vec![Stmt::Expression(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Variable("name".to_string()).into()
                })]
            })
        )
    }

    #[test]
    fn it_fails_for_broken_grouping() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::LeftParen,
                line: 1,
            },
            Token {
                value: TokenType::True,
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        let res = parser.parse().unwrap_err();
        assert_eq!(res.len(), 1);
        assert_eq!(
            res.first().unwrap(),
            &ParserError::MissingRParen {
                token: Token {
                    value: TokenType::Semicolon,
                    line: 1
                }
            }
        )
    }

    #[test]
    fn it_fails_for_missing_semi_in_print() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Print,
                line: 1,
            },
            Token {
                value: TokenType::True,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        let res = parser.parse().unwrap_err();
        assert_eq!(res.len(), 1);
        assert_eq!(
            res.first().unwrap(),
            &ParserError::MissingSemiColon {
                token: Token {
                    value: TokenType::Eof,
                    line: 1
                }
            }
        )
    }

    #[test]
    fn it_fails_for_invalid_assignment() {
        let parser = Parser::new(vec![
            Token {
                value: TokenType::Number("3".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Equal,
                line: 1,
            },
            Token {
                value: TokenType::Number("5".to_string()),
                line: 1,
            },
            Token {
                value: TokenType::Semicolon,
                line: 1,
            },
            Token {
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        let res = parser.parse().unwrap_err();
        assert_eq!(res.len(), 1);
        assert_eq!(
            res.first().unwrap(),
            &ParserError::InvalidAssignmentTarget {
                token: Token {
                    value: TokenType::Equal,
                    line: 1
                }
            }
        )
    }
}
