use std::{fmt::Display, iter, vec};

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
    pub line: usize,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ {}", self.value, self.line)
    }
}

#[derive(Debug, PartialEq)]
pub enum ScannerError {
    UnexpectedCharacter { c: char, line: usize },
    UnterminatedString { line: usize },
    // InvalidOperator { s: String, line: usize },
}

impl Display for ScannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScannerError::UnexpectedCharacter { c, line } => {
                write!(f, "Unexpected character \"{c}\" on line {line}")
            }
            ScannerError::UnterminatedString { line } => {
                write!(f, "Unterminated string starting on line {line}")
            }
        }
    }
}

pub struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    tokens: Vec<Token>,
    errors: Vec<ScannerError>,
    line: usize,
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
                '"' => {
                    // > a string literal!

                    // a string literal! consume until we end or hit another quote
                    // multi-line strings are supported
                    let val: String =
                        iter::from_fn(|| self.chars.next_if(|nc| *nc != '"')).collect();
                    // `next_if` doesn't consume the last character, so we need to manually advance past the closing quote, if present

                    match self.chars.next() {
                        // correctly terminated string!
                        Some('"') => {
                            // we advanced as many lines as there are newlines in the string
                            self.line += val.chars().filter(|c| *c == '\n').count();
                            self.add_token(TokenType::String(val));
                        }
                        Some(_) => panic!(
                            "ended a string on neither a doublequote or the end of the stream??"
                        ),
                        None => self
                            .errors
                            .push(ScannerError::UnterminatedString { line: self.line }),
                    };
                }
                _ if c.is_ascii_digit() => {
                    // > an int or float!

                    // this is the first or only part of the number
                    // special handling for `c` because we've already consumed it to get here
                    // so we have to make sure to include it in the result
                    let val = format!("{c}{}", self.take_digits());

                    match self.chars.peek() {
                        // a float! or a method call, which this doesn't handle well
                        Some('.') => {
                            self.chars.next(); // consume the `.`
                            let second_half = self.take_digits();
                            self.add_token(TokenType::Number(format!("{val}.{second_half}")));
                        }
                        // the end of the number (or the source itself)
                        _ => {
                            self.add_token(TokenType::Number(val));
                        }
                    };
                }
                _ if c.is_ascii_alphabetic() || c == '_' => {
                    // > a keyword or idenitifier!

                    let val = format!(
                        "{c}{}",
                        iter::from_fn(|| self
                            .chars
                            .next_if(|nc| nc.is_ascii_alphabetic() || *nc == '_'))
                        .collect::<String>()
                    );

                    match val.as_str() {
                        "and" => self.add_token(TokenType::And),
                        "class" => self.add_token(TokenType::Class),
                        "else" => self.add_token(TokenType::Else),
                        "false" => self.add_token(TokenType::False),
                        "for" => self.add_token(TokenType::For),
                        "fun" => self.add_token(TokenType::Fun),
                        "if" => self.add_token(TokenType::If),
                        "nil" => self.add_token(TokenType::Nil),
                        "or" => self.add_token(TokenType::Or),
                        "print" => self.add_token(TokenType::Print),
                        "return" => self.add_token(TokenType::Return),
                        "super" => self.add_token(TokenType::Super),
                        "this" => self.add_token(TokenType::This),
                        "true" => self.add_token(TokenType::True),
                        "var" => self.add_token(TokenType::Var),
                        "while" => self.add_token(TokenType::While),
                        _ => self.add_token(TokenType::Identifier(val)),
                    }
                }
                ' ' | '\r' | '\t' => {}
                '\n' => self.line += 1,
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

    fn add_token(&mut self, value: TokenType) {
        self.tokens.push(Token {
            value,
            line: self.line,
        });
    }

    /** Adds one of two tokens if the next character is a specific value. Only consumes a character if there's a match */
    fn add_compound_token(&mut self, if_next_is: &char, then: TokenType, otherwise: TokenType) {
        if self.chars.peek() == Some(if_next_is) {
            self.add_token(then);
            self.chars.next();
        } else {
            self.add_token(otherwise);
        }
    }

    /** Consume `chars` until you reach a non-digit character and return the resulting String */
    fn take_digits(&mut self) -> String {
        iter::from_fn(|| self.chars.next_if(|nc| nc.is_ascii_digit())).collect()
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
                text: "!=!;== =<<>=> <=!!!".to_string(),
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

    #[test]
    fn ignored_whitespace() {
        assert_eq!(
            tokenize(&Source {
                text: "!  =  +  - \n! . * \t\t ; \n /".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Equal,
                        line: 1
                    },
                    Token {
                        value: TokenType::Plus,
                        line: 1
                    },
                    Token {
                        value: TokenType::Minus,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 2
                    },
                    Token {
                        value: TokenType::Dot,
                        line: 2
                    },
                    Token {
                        value: TokenType::Star,
                        line: 2
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 2
                    },
                    Token {
                        value: TokenType::Slash,
                        line: 3
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 3
                    },
                ]
            })
        );
    }

    #[test]
    fn basic_string() {
        assert_eq!(
            tokenize(&Source {
                text: "!!\"neat\";".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::String("neat".to_string()),
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
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
    fn multiline_string() {
        assert_eq!(
            tokenize(&Source {
                text: "!\"ne\na\nt\";\n;".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::String("ne\na\nt".to_string()),
                        // strings are attributed to the line on which they end?
                        line: 3
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 3
                    },
                    Token {
                        value: TokenType::Semicolon,
                        line: 4
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 4
                    },
                ]
            })
        );
    }

    #[test]
    fn integers() {
        assert_eq!(
            tokenize(&Source {
                text: "!123;".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Number("123".to_string()),
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
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
    fn floats() {
        assert_eq!(
            tokenize(&Source {
                text: "!123.456;".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Bang,
                        line: 1
                    },
                    Token {
                        value: TokenType::Number("123.456".to_string()),
                        line: 1
                    },
                    Token {
                        value: TokenType::Semicolon,
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
    fn int_at_end_of_input() {
        assert_eq!(
            tokenize(&Source {
                text: "123".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Number("123".to_string()),
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
    fn float_at_end_of_input() {
        assert_eq!(
            tokenize(&Source {
                text: "123.456".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Number("123.456".to_string()),
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
    fn keywords() {
        assert_eq!(
            tokenize(&Source {
                text: "123.456".to_string(),
            }),
            Ok(Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Number("123.456".to_string()),
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
    fn unterminated_string() {
        assert_eq!(
            tokenize(&Source {
                text: "\"neat".to_string(),
            }),
            Err(vec![ScannerError::UnterminatedString { line: 1 }])
        );
    }

    #[test]
    fn unrecognized_character() {
        assert_eq!(
            tokenize(&Source {
                // @ is never used
                text: "!+@".to_string(),
            }),
            Err(vec![ScannerError::UnexpectedCharacter { c: '@', line: 1 }])
        );
    }
}
