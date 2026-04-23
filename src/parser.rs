use std::fmt::Display;

use crate::ast::{Ast, Expr, Literal};
use crate::scanner::{Token, TokenType, Tokens};

#[derive(Debug, PartialEq)]
pub enum ParserError {
    MissingRParen { token: Token },
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::MissingRParen { token } => {
                write!(f, "[{token}] Missing ')' after expression.",)
            }
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    errors: Vec<ParserError>,
    current: usize,
}

// grammar (different from book):

// expression   := binary | term ;
// binary       := term operator term ;
// term         := literal | unary | grouping ;
// unary        := ("-" | "!") term ;
// literal      := NUMBER | STRING | "true" | "false" | "nil" ;
// grouping     := "(" expression ")" ;
// operator     :=  "==" | "!=" | "<" | "<=" | ">" | ">="
//                 | "+" | "-"  | "*" | "/" ;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            errors: vec![],
            current: 0,
        }
    }

    pub fn parse(mut self) -> Result<Ast, Vec<ParserError>> {
        let root = self.parse_expression();

        if self.errors.is_empty() {
            if let Some(root) = root {
                Ok(Ast { root })
            } else {
                panic!("no errors, but an empty root node?")
            }
        } else {
            Err(self.errors)
        }
    }

    fn parse_expression(&mut self) -> Option<Expr> {
        self.parse_binary().or_else(|| self.parse_term())
    }

    fn parse_binary(&mut self) -> Option<Expr> {
        let cur = self.current;
        if let Some(left) = self.parse_term() {
            // to be valid binary, the next bit has to be a binary operator
            if matches!(
                self.peek().value,
                TokenType::EqualEqual
                    | TokenType::BangEqual
                    | TokenType::LessThan
                    | TokenType::LessThanEqual
                    | TokenType::GreaterThan
                    | TokenType::GreaterThanEqual
                    | TokenType::Plus
                    | TokenType::Minus
                    | TokenType::Star
                    | TokenType::Slash
            ) {
                // TODO: if I used &str in my tokens, I'd be able to copy here
                // there aren't _that_ many places I'd ned to add need lifetimes
                let op = self.consume().value.clone().into();

                if let Some(right) = self.parse_term() {
                    return Some(Expr::Binary {
                        left: left.into(),
                        op,
                        right: right.into(),
                    });
                }
            }
        }

        // unroll if we didn't find a binary expression
        self.current = cur;
        None
    }

    fn parse_term(&mut self) -> Option<Expr> {
        self.parse_literal()
            .or_else(|| self.parse_unary())
            .or_else(|| self.parse_grouping())
    }

    fn parse_literal(&mut self) -> Option<Expr> {
        let maybe_literal = match &self.peek().value {
            TokenType::Nil => Some(Literal::Nil),
            TokenType::True => Some(Literal::True),
            TokenType::False => Some(Literal::False),
            TokenType::String(s) => Some(Literal::String(s.to_owned())),
            TokenType::Number(s) => Some(Literal::Number(
                s.parse().expect("expected {s} to be a valid f64"),
            )),
            _ => None,
        };

        if let Some(literal) = maybe_literal {
            self.consume();
            Some(Expr::Literal(literal))
        } else {
            None
        }
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.consume_if(TokenType::Bang) || self.consume_if(TokenType::Minus) {
            let op = self.previous().value.clone().into();
            match self.parse_term() {
                Some(term) => Some(Expr::Unary {
                    op,
                    expr: term.into(),
                }),
                None => todo!("don't think we can hit this?"),
            }
        } else {
            None
        }
    }

    fn parse_grouping(&mut self) -> Option<Expr> {
        if self.consume_if(TokenType::LeftParen) {
            let ex = self.parse_expression();
            if let Some(expr) = ex
                && self.consume_if(TokenType::RightParen)
            {
                Some(Expr::Grouping(expr.into()))
            } else {
                self.errors.push(ParserError::MissingRParen {
                    token: (*self.peek()).clone(),
                });
                None
            }
        } else {
            None
        }
    }

    // HELPERS

    /** book calls this `match`, but I don't like that it doesn't communicate that it advances the pointer */
    // TODO: option? we always call .previous() right after
    fn consume_if(&mut self, token_type: TokenType) -> bool {
        if self.peek().value == token_type {
            self.consume();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek().value == TokenType::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn consume(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Literal(Literal::Nil)
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Literal(Literal::False)
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Literal(Literal::String("cool".to_string()))
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Literal(Literal::Number(123.0))
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Literal(Literal::Number(123.456))
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Grouping(Expr::Literal(Literal::True).into())
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Expr::Literal(Literal::Number(3.0)).into()
                }
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Binary {
                    left: Expr::Literal(Literal::Number(1.0)).into(),
                    op: BinaryOp::Div,
                    right: Expr::Literal(Literal::Number(2.0)).into()
                }
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Expr::Unary {
                        op: UnaryOp::Not,
                        expr: Expr::Literal(Literal::Number(2.0)).into()
                    }
                    .into()
                }
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Binary {
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
                }
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser.parse(),
            Ok(Ast {
                root: Expr::Binary {
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
                }
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
                value: TokenType::Eof,
                line: 1,
            },
        ]);
        assert_eq!(
            parser
                .parse()
                .expect_err("expected an error")
                // my code returns two errors, since i don't sync well/)
                .first()
                .expect("expected an element"),
            &ParserError::MissingRParen {
                token: Token {
                    value: TokenType::Eof,
                    line: 1
                }
            }
        )
    }
}
