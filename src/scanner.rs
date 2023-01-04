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

    pub fn json(self) -> impl Iterator<Item = ScanResult<'a>> {
        self.into_iter().filter(|event| {
            if let Ok(event) = event {
                match event.token {
                    Token::BlockComment(_) | Token::LineComment(_) | Token::Newline => {
                        return false
                    }
                    _ => {}
                }
            }
            true
        })
    }

    fn parse_value(&mut self) -> Option<ScanResult<'a>> {
        self.skip_whitespace();
        if let Some((i, c)) = self.next_char() {
            let start = self.current_idx;
            match c {
                '\n' => Some(Ok(Event {
                    token: Token::Newline,
                    range: start..(start + 1),
                })),
                '{' => Some(Ok(Event {
                    token: Token::ObjectStart,
                    range: start..(start + 1),
                })),
                '}' => Some(Ok(Event {
                    token: Token::ObjectEnd,
                    range: start..(start + 1),
                })),
                '[' => Some(Ok(Event {
                    token: Token::ArrayStart,
                    range: start..(start + 1),
                })),
                ']' => Some(Ok(Event {
                    token: Token::ArrayEnd,
                    range: start..(start + 1),
                })),
                ',' => Some(Ok(Event {
                    token: Token::Comma,
                    range: start..(start + 1),
                })),
                ':' => Some(Ok(Event {
                    token: Token::Colon,
                    range: start..(start + 1),
                })),
                'n' => Some(self.parse_null(start)),
                't' => Some(self.parse_bool_true(start)),
                'f' => Some(self.parse_bool_false(start)),
                '/' => Some(self.parse_comment(start)),
                '"' => Some(self.parse_string(start)),
                c => {
                    if c.is_numeric() || c == '-' {
                        Some(self.parse_number(start))
                    } else {
                        Some(Err(Error::UnexpectedCharacter((i, c))))
                    }
                }
            }
        } else {
            None
        }
    }

    fn parse_number(&mut self, start: usize) -> ScanResult<'a> {
        // TODO(ryanfowler): Parse and validate a number properly.
        let mut end = start + 1;
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

    fn parse_string(&mut self, start: usize) -> ScanResult<'a> {
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

    fn parse_comment(&mut self, start: usize) -> ScanResult<'a> {
        match self.next_char() {
            Some((_, '/')) => self.parse_line_comment(start),
            Some((_, '*')) => self.parse_block_comment(start),
            Some(v) => Err(Error::UnexpectedCharacter(v)),
            None => Err(Error::UnexpectedEOF),
        }
    }

    fn parse_line_comment(&mut self, start: usize) -> ScanResult<'a> {
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

    fn parse_block_comment(&mut self, start: usize) -> ScanResult<'a> {
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

    fn parse_null(&mut self, start: usize) -> ScanResult<'a> {
        if self.next_chars_equal("ull") {
            Ok(Event {
                token: Token::Null,
                range: start..(start + 4),
            })
        } else {
            Err(Error::UnexpectedCharacter((start, 'n')))
        }
    }

    fn parse_bool_true(&mut self, start: usize) -> ScanResult<'a> {
        if self.next_chars_equal("rue") {
            Ok(Event {
                token: Token::Bool(true),
                range: start..(start + 4),
            })
        } else {
            Err(Error::UnexpectedCharacter((start, 't')))
        }
    }

    fn parse_bool_false(&mut self, start: usize) -> ScanResult<'a> {
        if self.next_chars_equal("alse") {
            Ok(Event {
                token: Token::Bool(false),
                range: start..(start + 5),
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
