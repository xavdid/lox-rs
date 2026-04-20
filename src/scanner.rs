use std::{fmt::Display, vec};

use crate::reader::Source;

#[derive(Debug, PartialEq)]
pub struct Tokens {
    pub tokens: Vec<Token>,
}

#[derive(Debug, PartialEq)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    // adding a value may be a mistake here, but we'll see!
    Identifier(String),
    String(String),
    Number(String),

    // Keywords.
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Eof,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub type_: TokenType,
    // pub value: String, // i'm going rogue
    pub line: u32,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ {}", self.type_, self.line)
    }
}

#[derive(Debug, PartialEq)]
pub enum ScannerError {
    UnexpectedCharacter { c: char, line: u32 },
    // InvalidOperator { s: String, line: u32 },
}

impl Display for ScannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScannerError::UnexpectedCharacter { c, line } => {
                write!(f, "Unexpected character \"{c}\" on line {line}")
            }
        }
    }
}

pub struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    tokens: Vec<Token>,
    errors: Vec<ScannerError>,
    line: u32,
}

impl Scanner<'_> {
    fn new(source: &Source) -> Scanner<'_> {
        Scanner {
            chars: source.text.chars().peekable(),
            tokens: vec![],
            errors: vec![],
            line: 1,
        }
    }

    fn add_token(&mut self, type_: TokenType) {
        self.tokens.push(Token {
            type_,
            line: self.line,
        });
    }

    fn add_token_if_eq(&mut self, if_next_is: &char, then: TokenType, otherwise: TokenType) {
        self.chars.next();
        if self.chars.peek() == Some(if_next_is) {
            self.add_token(then);
        } else {
            self.add_token(otherwise);
        }
    }

    fn scan_tokens(mut self) -> Result<Tokens, Vec<ScannerError>> {
        while let Some(&c) = self.chars.peek() {
            println!("tokenizing: {c}");
            match c {
                '(' => self.add_token(TokenType::LeftParen),
                ')' => self.add_token(TokenType::RightParen),
                '{' => self.add_token(TokenType::LeftBrace),
                '}' => self.add_token(TokenType::RightBrace),
                ',' => self.add_token(TokenType::Comma),
                '.' => self.add_token(TokenType::Dot),
                '-' => self.add_token(TokenType::Minus),
                '+' => self.add_token(TokenType::Plus),
                ';' => self.add_token(TokenType::Semicolon),
                '*' => self.add_token(TokenType::Star),
                '!' => self.add_token_if_eq(&'=', TokenType::BangEqual, TokenType::Bang),
                _ => {
                    self.errors
                        .push(ScannerError::UnexpectedCharacter { c, line: self.line });
                }
            }
            self.chars.next();
        }

        self.tokens.push(Token {
            type_: TokenType::Eof,
            line: self.line,
        });

        if self.errors.is_empty() {
            Ok(Tokens {
                tokens: self.tokens,
            })
        } else {
            Err(self.errors)
        }
    }
}

pub fn tokenize(source: &Source) -> Result<Tokens, Vec<ScannerError>> {
    println!("Tokenizing!");

    Scanner::new(source).scan_tokens()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            tokenize(&Source {
                text: ";(){}*;;".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        type_: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        type_: TokenType::LeftParen,
                        line: 1
                    },
                    Token {
                        type_: TokenType::RightParen,
                        line: 1
                    },
                    Token {
                        type_: TokenType::LeftBrace,
                        line: 1
                    },
                    Token {
                        type_: TokenType::RightBrace,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Star,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Eof,
                        line: 1
                    },
                ]
            })
        );
    }

    #[test]
    fn multi_character_tokens() {
        assert_eq!(
            tokenize(&Source {
                text: "!=!".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        type_: TokenType::BangEqual,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        type_: TokenType::Eof,
                        line: 1
                    },
                ]
            })
        );
    }
}
