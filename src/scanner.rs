use anyhow::{Result, anyhow};
use std::{fmt::Display, iter, vec};

use crate::{join_errors, reader::Source};

#[derive(Debug, PartialEq)]
pub struct Tokens {
    pub tokens: Vec<Token>,
}

#[derive(Debug, PartialEq, Clone)]
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
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,

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

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub value: TokenType,
    // pub value: String, // i'm going rogue
    pub line: usize,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ line {}", self.value, self.line)
    }
}

pub struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    tokens: Vec<Token>,
    errors: Vec<anyhow::Error>,
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

    fn scan_tokens(mut self) -> Result<Tokens, Vec<anyhow::Error>> {
        while let Some(c) = self.chars.next() {
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
                '<' => self.add_compound_token(&'=', TokenType::LessThanEqual, TokenType::LessThan),
                '>' => self.add_compound_token(
                    &'=',
                    TokenType::GreaterThanEqual,
                    TokenType::GreaterThan,
                ),
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
                '"' => self.add_string(),
                _ if c.is_ascii_digit() => self.add_number(c),
                _ if is_ident(c) => {
                    // > a keyword or idenitifier!

                    let val = self.take_ident(c);
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
                    self.new_err(&format!("Unexpected character \"{c}\"."));
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

    /// Adds one of two tokens if the next character is a specific value. Only consumes a character if there's a match
    fn add_compound_token(&mut self, if_next_is: &char, then: TokenType, otherwise: TokenType) {
        if self.chars.peek() == Some(if_next_is) {
            self.add_token(then);
            self.chars.next();
        } else {
            self.add_token(otherwise);
        }
    }

    /// Consume `chars` until you reach a non-digit character and return the resulting String
    fn take_digits(&mut self, starting_with: char) -> String {
        let mut res = String::from(starting_with);
        res.extend(iter::from_fn(|| {
            self.chars.next_if(|nc| nc.is_ascii_digit())
        }));
        res
    }

    /// Consume `chars` until you reach a non-ident character and return the resulting String
    fn take_ident(&mut self, starting_with: char) -> String {
        let mut res = String::from(starting_with);
        res.extend(iter::from_fn(|| self.chars.next_if(|c| is_ident(*c))));
        res
    }

    /// add a number token, consuming what it needs
    fn add_number(&mut self, c: char) {
        // this is the first or only part of the number
        // special handling for `c` because we've already consumed it to get here
        // so we have to make sure to include it in the result
        let mut val = self.take_digits(c);

        match self.chars.peek() {
            // a float! or a method call, which this doesn't handle well
            Some('.') => {
                let separator = self.chars.next().unwrap(); // consume the `.`
                val.push_str(&self.take_digits(separator)); // but include it in the result
                if val.ends_with('.') {
                    self.new_err(&format!("Got float with no decimals: '{val}'"));
                } else {
                    self.add_token(TokenType::Number(val));
                }
            }
            // the end of the number (or the source itself)
            _ => {
                self.add_token(TokenType::Number(val));
            }
        };
    }

    /// add a string token, consuming what it needs. Will eat all further syntax errors, since it scans the rest of the input looking for a closing quote
    fn add_string(&mut self) {
        // consume until we end or hit another quote
        // multi-line strings are supported
        let val: String = iter::from_fn(|| self.chars.next_if(|nc| *nc != '"')).collect();

        // `next_if` doesn't consume the last character, so we need to manually advance past the closing quote, if present
        match self.chars.next() {
            // correctly terminated string!
            Some('"') => {
                // we advanced as many lines as there are newlines in the string
                self.line += val.chars().filter(|c| *c == '\n').count();
                self.add_token(TokenType::String(val));
            }
            Some(_) => panic!(
                "ended a string on neither a doublequote or the end of the stream?? Shouldn't happen"
            ),
            None => self.new_err("Unterminated string started"),
        };
    }

    fn new_err(&mut self, msg: &str) {
        self.errors.push(anyhow!("[line: {}] {msg}", self.line))
    }
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

pub fn tokenize(source: &Source) -> Result<Tokens> {
    Scanner::new(source).scan_tokens().map_err(join_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_contains;
    use pretty_assertions::assert_eq;

    fn scan(text: &str) -> Tokens {
        Scanner::new(&Source {
            text: text.to_string(),
        })
        .scan_tokens()
        .expect("this method should return Ok()")
    }
    fn fail_scan(text: &str) -> Vec<anyhow::Error> {
        Scanner::new(&Source {
            text: text.to_string(),
        })
        .scan_tokens()
        .expect_err("errors method expects errors")
    }

    #[test]
    fn it_works() {
        assert_eq!(
            scan(";(){}*;;+*-.,"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn multi_character_tokens() {
        assert_eq!(
            scan("!=!;== =<<>=> <=!!!"),
            Tokens {
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
                        value: TokenType::LessThan,
                        line: 1
                    },
                    Token {
                        value: TokenType::LessThan,
                        line: 1
                    },
                    Token {
                        value: TokenType::GreaterThanEqual,
                        line: 1
                    },
                    Token {
                        value: TokenType::GreaterThan,
                        line: 1
                    },
                    Token {
                        value: TokenType::LessThanEqual,
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
            }
        );
    }

    #[test]
    fn comments_basics() {
        assert_eq!(
            scan("!/!// ignored\n/!"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn comment_no_trailing_newline() {
        assert_eq!(
            scan("!/!// ignored"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn ignored_whitespace() {
        assert_eq!(
            scan("!  =  +  - \n! . * \t\t ; \n /"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn basic_string() {
        assert_eq!(
            scan("!!\"neat\";"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn unicode_string() {
        assert_eq!(
            scan("!!\"jalapeño\";"),
            Tokens {
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
                        value: TokenType::String("jalapeño".to_string()),
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
            }
        );
    }

    #[test]
    fn multiline_string() {
        assert_eq!(
            scan("!\"ne\na\nt\";\n;"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn multiline_int_not_a_thing() {
        assert_eq!(
            scan("123\n456"),
            Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Number("123".to_string()),
                        line: 1
                    },
                    Token {
                        value: TokenType::Number("456".to_string()),
                        line: 2
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 2
                    },
                ]
            }
        );
    }

    #[test]
    fn integers() {
        assert_eq!(
            scan("!123;"),
            Tokens {
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
            }
        );
    }
    #[test]
    fn floats() {
        assert_eq!(
            scan("!123.456;"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn int_at_end_of_input() {
        assert_eq!(
            scan("123"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn float_at_end_of_input() {
        assert_eq!(
            scan("123.456"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn num_leading_period() {
        // this is a weird one- I think i'm diverging from the book
        assert_eq!(
            scan(".456"),
            Tokens {
                tokens: vec![
                    Token {
                        value: TokenType::Dot,
                        line: 1
                    },
                    Token {
                        value: TokenType::Number("456".to_string()),
                        line: 1
                    },
                    Token {
                        value: TokenType::Eof,
                        line: 1
                    },
                ]
            }
        );
    }

    #[test]
    fn keywords() {
        assert_eq!(
            scan("123.456"),
            Tokens {
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
            }
        );
    }

    #[test]
    fn num_trailing_period() {
        let errors = fail_scan("123.");
        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "float with no decimals");
    }

    #[test]
    fn unterminated_string() {
        let errors = fail_scan("\"neat");
        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "unterminated string");
    }

    #[test]
    fn unrecognized_character() {
        let errors = fail_scan("!+@");
        assert_eq!(errors.len(), 1);
        assert_contains(&errors[0], "unexpected character");
        assert_contains(&errors[0], "@");
        assert_contains(&errors[0], "[line: 1]");
    }

    #[test]
    fn multiple_errors() {
        let errors = fail_scan("!+@\n12.");
        assert_eq!(errors.len(), 2);

        assert_contains(&errors[0], "@");
        assert_contains(&errors[0], "[line: 1]");
        assert_contains(&errors[1], "decimals");
        assert_contains(&errors[1], "[line: 2]");
    }

    #[test]
    fn unterminated_takes_priority() {
        let errors = fail_scan("\"a!+@\n12.");
        assert_eq!(errors.len(), 1);

        assert_contains(&errors[0], "unterminated string");
        assert_contains(&errors[0], "[line: 1]");
    }
}
