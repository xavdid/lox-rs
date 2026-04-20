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

// pub struct Scanner {
//     chars:
//     tokens: Vec<Token>,
//     line: u32,
// }

// impl Scanner {
//     fn new() -> Scanner {
//         Scanner {
//             tokens: vec![],
//             line: 1,
//         }
//     }
// }

pub fn tokenize(source: &Source) -> Result<Tokens, Vec<ScannerError>> {
    println!("Tokenizing!");

    let mut tokens = vec![];
    let mut errors = vec![];

    // let mut start = 0;
    // let mut current = 0;
    let mut line = 1;

    // let source_len = source.text.len();
    let mut chars = source.text.chars().peekable();

    let mut add_token = |type_: TokenType| {
        tokens.push(Token { type_, line });
    };
    let mut add_conditionally = |if_next_is: &char, then: TokenType, otherwise: TokenType| {
        chars.next();
        if chars.peek() == Some(if_next_is) {
            add_token(then);
        } else {
            add_token(otherwise);
        }
    };

    while let Some(&c) = chars.peek() {
        println!("tokenizing: {c}");
        match c {
            '(' => add_token(TokenType::LeftParen),
            ')' => add_token(TokenType::RightParen),
            '{' => add_token(TokenType::LeftBrace),
            '}' => add_token(TokenType::RightBrace),
            ',' => add_token(TokenType::Comma),
            '.' => add_token(TokenType::Dot),
            '-' => add_token(TokenType::Minus),
            '+' => add_token(TokenType::Plus),
            ';' => add_token(TokenType::Semicolon),
            '*' => add_token(TokenType::Star),
            '!' => {
                chars.next();
                add_conditionally(&'=', TokenType::BangEqual, TokenType::Bang);
            }
            _ => {
                errors.push(ScannerError::UnexpectedCharacter { c, line });
            }
        }
        chars.next();
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    tokens.push(Token {
        type_: TokenType::Eof,
        line,
    });
    Ok(Tokens { tokens })
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
                text: "!=".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        type_: TokenType::BangEqual,
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
