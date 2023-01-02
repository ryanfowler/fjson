use std::{iter::Peekable, ops::Range, str::CharIndices};

use crate::error::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct Event<'a> {
    pub token: Token<'a>,
    pub range: Range<usize>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
    Newline,
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    Comma,
    Colon,
    Null,
    LineComment(&'a str),
    BlockComment(&'a str),
    String(&'a str),
    Number(&'a str),
    Bool(bool),
}

pub type ScanResult<'a> = Result<Event<'a>, Error>;

pub struct Scanner<'a> {
    input: &'a str,
    current_idx: usize,
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Iterator for Scanner<'a> {
    type Item = ScanResult<'a>;

    fn next(&mut self) -> Option<ScanResult<'a>> {
        self.parse_value()
    }
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Scanner {
            input,
            current_idx: 0,
            chars: input.char_indices().peekable(),
        }
    }

    fn parse_value(&mut self) -> Option<ScanResult<'a>> {
        self.skip_whitespace();
        if let Some(&(i, c)) = self.peek_char() {
            match c {
                '\n' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::Newline,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                '{' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::ObjectStart,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                '}' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::ObjectEnd,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                '[' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::ArrayStart,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                ']' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::ArrayEnd,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                ',' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::Comma,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                ':' => {
                    self.skip_char();
                    Some(Ok(Event {
                        token: Token::Colon,
                        range: self.current_idx..(self.current_idx + 1),
                    }))
                }
                'n' => Some(self.parse_null()),
                't' => Some(self.parse_bool_true()),
                'f' => Some(self.parse_bool_false()),
                '/' => Some(self.parse_comment()),
                '"' => Some(self.parse_string()),
                c => {
                    if c.is_numeric() || c == '-' {
                        Some(self.parse_number())
                    } else {
                        Some(Err(Error::UnexpectedCharacter((i, c))))
                    }
                }
            }
        } else {
            None
        }
    }

    fn parse_number(&mut self) -> ScanResult<'a> {
        // TODO(ryanfowler): Parse and validate a number properly.
        let start = self.current_idx + 1;

        match self.next_char() {
            None => {
                return Err(Error::UnexpectedEOF);
            }
            Some((i, c)) => {
                if c != '-' && !c.is_numeric() {
                    return Err(Error::UnexpectedCharacter((i, c)));
                }
            }
        }

        let mut end = self.current_idx;
        while let Some(&(i, c)) = self.peek_char() {
            end = i;
            if c.is_numeric() || c == 'e' || c == 'E' || c == '+' {
                self.skip_char();
            } else {
                break;
            }
        }

        let range = start..end;
        Ok(Event {
            token: Token::Number(&self.input[range.clone()]),
            range,
        })
    }

    fn parse_string(&mut self) -> ScanResult<'a> {
        match self.next_char() {
            Some((_, '"')) => {}
            Some((i, c)) => return Err(Error::UnexpectedCharacter((i, c))),
            None => return Err(Error::UnexpectedEOF),
        }

        let start = self.current_idx;
        while let Some((_, c)) = self.next_char() {
            match c {
                '\\' => match self.next_char() {
                    Some((i, c)) => match c {
                        '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' => {}
                        'u' => {
                            for _ in 0..4 {
                                match self.next_char() {
                                    Some((i, c)) => {
                                        if !c.is_ascii_hexdigit() {
                                            return Err(Error::UnexpectedCharacter((i, c)));
                                        }
                                    }
                                    None => return Err(Error::UnexpectedEOF),
                                }
                            }
                        }
                        c => return Err(Error::UnexpectedCharacter((i, c))),
                    },
                    None => return Err(Error::UnexpectedEOF),
                },
                '"' => {
                    let end = self.current_idx;
                    return Ok(Event {
                        token: Token::String(&self.input[(start + 1)..end]),
                        range: start..(end + 1),
                    });
                }
                _ => {}
            }
        }
        Err(Error::UnexpectedEOF)
    }

    fn parse_comment(&mut self) -> ScanResult<'a> {
        match self.next_char() {
            Some((_, '/')) => match self.next_char() {
                Some((_, '/')) => self.parse_line_comment(),
                Some((_, '*')) => self.parse_block_comment(),
                Some(v) => Err(Error::UnexpectedCharacter(v)),
                None => Err(Error::UnexpectedEOF),
            },
            Some(v) => Err(Error::UnexpectedCharacter(v)),
            None => Err(Error::UnexpectedEOF),
        }
    }

    fn parse_line_comment(&mut self) -> ScanResult<'a> {
        let start = self.current_idx - 1;
        let mut end = start + 2;
        while let Some(&(i, c)) = self.peek_char() {
            end = i;
            if c == '\n' {
                break;
            } else if c == '\r' {
                self.skip_char();
                if let Some(&(_, c)) = self.peek_char() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            } else {
                self.skip_char();
            }
        }
        Ok(Event {
            token: Token::LineComment(&self.input[(start + 2..end)]),
            range: start..end,
        })
    }

    fn parse_block_comment(&mut self) -> ScanResult<'a> {
        let start = self.current_idx - 1;
        while let Some((_, c)) = self.next_char() {
            if c == '*' {
                if let Some(&(i, '/')) = self.peek_char() {
                    self.skip_char();
                    return Ok(Event {
                        token: Token::BlockComment(&self.input[(start + 2)..(i - 1)]),
                        range: start..(i + 1),
                    });
                }
            }
        }
        Err(Error::UnexpectedEOF)
    }

    fn parse_null(&mut self) -> ScanResult<'a> {
        let start = self.current_idx;
        if self.next_chars_equal("null") {
            Ok(Event {
                token: Token::Null,
                range: start..(start + 4),
            })
        } else {
            Err(Error::UnexpectedCharacter((start, 'n')))
        }
    }

    fn parse_bool_true(&mut self) -> ScanResult<'a> {
        let start = self.current_idx;
        if self.next_chars_equal("true") {
            Ok(Event {
                token: Token::Bool(true),
                range: start..(start + 4),
            })
        } else {
            Err(Error::UnexpectedCharacter((start, 't')))
        }
    }

    fn parse_bool_false(&mut self) -> ScanResult<'a> {
        let start = self.current_idx;
        if self.next_chars_equal("false") {
            Ok(Event {
                token: Token::Bool(false),
                range: start..(start + 4),
            })
        } else {
            Err(Error::UnexpectedCharacter((start, 'f')))
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.1.is_whitespace() && c.1 != '\n' {
                self.skip_char();
            } else {
                return;
            }
        }
    }

    fn next_chars_equal(&mut self, s: &str) -> bool {
        for ch in s.chars() {
            match self.next_char() {
                Some((_, c)) => {
                    if ch != c {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            }
        }
        true
    }

    fn next_char(&mut self) -> Option<(usize, char)> {
        if let Some((i, c)) = self.chars.next() {
            self.current_idx = i;
            Some((i, c))
        } else {
            None
        }
    }

    fn skip_char(&mut self) {
        self.next_char();
    }

    fn peek_char(&mut self) -> Option<&(usize, char)> {
        self.chars.peek()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner() {
        let input = r#"{
            // This is a comment.
            "key1": "val1",
            "key2": 100,
            /*
             * This is a block comment.
             */
            "key3":[        "1", 2, {}  ]
        }"#;
        let scanner = Scanner::new(input);
        println!("{}", input);
        for token in scanner {
            match token {
                Err(err) => panic!("parsing error: {:?}", err),
                Ok(token) => {
                    println!("{:?}", token);
                }
            }
        }
    }
}
