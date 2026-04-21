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
    pub value: TokenType,
    // pub value: String, // i'm going rogue
    pub line: u32,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ {}", self.value, self.line)
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

    fn add_token(&mut self, value: TokenType) {
        self.tokens.push(Token {
            value,
            line: self.line,
        });
    }

    fn add_compound_token(&mut self, if_next_is: &char, then: TokenType, otherwise: TokenType) {
        if self.chars.peek() == Some(if_next_is) {
            self.add_token(then);
            self.chars.next();
        } else {
            self.add_token(otherwise);
        }
    }

    fn scan_tokens(mut self) -> Result<Tokens, Vec<ScannerError>> {
        while let Some(c) = self.chars.next() {
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
                '!' => self.add_compound_token(&'=', TokenType::BangEqual, TokenType::Bang),
                '=' => self.add_compound_token(&'=', TokenType::EqualEqual, TokenType::Equal),
                '<' => self.add_compound_token(&'=', TokenType::LessEqual, TokenType::Less),
                '>' => self.add_compound_token(&'=', TokenType::GreaterEqual, TokenType::Greater),
                // I don't have the book's `match` (yet)
                '/' => {
                    if self.chars.peek() == Some(&'/') {
                        // consume characters until we reach a newline (or end)
                        loop {
                            match self.chars.next() {
                                Some('\n') => {
                                    self.line += 1;
                                    break;
                                }
                                Some(_) => {}
                                None => break,
                            }
                        }
                    } else {
                        self.add_token(TokenType::Slash);
                    }
                }
                _ => {
                    self.errors
                        .push(ScannerError::UnexpectedCharacter { c, line: self.line });
                }
            }
        }

        self.tokens.push(Token {
            value: TokenType::Eof,
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
                text: ";(){}*;;+*-.,".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        value: TokenType::LeftParen,
                        line: 1
                    },
                    Token {
                        value: TokenType::RightParen,
                        line: 1
                    },
                    Token {
                        value: TokenType::LeftBrace,
                        line: 1
                    },
                    Token {
                        value: TokenType::RightBrace,
                        line: 1
                    },
                    Token {
                        value: TokenType::Star,
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        value: TokenType::Plus,
                        line: 1
                    },
                    Token {
                        value: TokenType::Star,
                        line: 1
                    },
                    Token {
                        value: TokenType::Minus,
                        line: 1
                    },
                    Token {
                        value: TokenType::Dot,
                        line: 1
                    },
                    Token {
                        value: TokenType::Comma,
                        line: 1
                    },
                    Token {
                        value: TokenType::Eof,
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
                text: "!=!;===<<>=><=!!!".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::BangEqual,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 1
                    },
                    Token {
                        value: TokenType::EqualEqual,
                        line: 1
                    },
                    Token {
                        value: TokenType::Equal,
                        line: 1
                    },
                    Token {
                        value: TokenType::Less,
                        line: 1
                    },
                    Token {
                        value: TokenType::Less,
                        line: 1
                    },
                    Token {
                        value: TokenType::GreaterEqual,
                        line: 1
                    },
                    Token {
                        value: TokenType::Greater,
                        line: 1
                    },
                    Token {
                        value: TokenType::LessEqual,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 1
                    },
                ]
            })
        );
    }

    #[test]
    fn comments_basics() {
        assert_eq!(
            tokenize(&Source {
                text: "!/!// ignored\n/!".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Slash,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Slash,
                        line: 2
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 2
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 2
                    },
                ]
            })
        );
    }

    #[test]
    fn comment_no_trailing_newline() {
        assert_eq!(
            tokenize(&Source {
                text: "!/!// ignored".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Slash,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 1
                    },
                ]
            })
        );
    }
}
