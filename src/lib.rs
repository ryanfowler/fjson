use std::{iter::Peekable, str::CharIndices};

use crate::error::Error;

mod error;
pub mod format;

//pub struct Event<'a> {
//    token: Token<'a>,
//    start: usize,
//    end: usize,
//}

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

impl Token<'_> {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Token::Newline => "\\n",
            Token::ObjectStart => "{",
            Token::ObjectEnd => "}",
            Token::ArrayStart => "[",
            Token::ArrayEnd => "]",
            Token::Comma => ",",
            Token::Colon => ":",
            Token::Null => "null",
            Token::LineComment(_) => "line comment",
            Token::BlockComment(_) => "block comment",
            Token::String(_) => "string",
            Token::Number(_) => "number",
            Token::Bool(_) => "bool",
        }
    }
}

type ScanResult<'a> = Result<Token<'a>, Error>;

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
                    Some(Ok(Token::Newline))
                }
                '{' => {
                    self.skip_char();
                    Some(Ok(Token::ObjectStart))
                }
                '}' => {
                    self.skip_char();
                    Some(Ok(Token::ObjectEnd))
                }
                '[' => {
                    self.skip_char();
                    Some(Ok(Token::ArrayStart))
                }
                ']' => {
                    self.skip_char();
                    Some(Ok(Token::ArrayEnd))
                }
                ',' => {
                    self.skip_char();
                    Some(Ok(Token::Comma))
                }
                ':' => {
                    self.skip_char();
                    Some(Ok(Token::Colon))
                }
                'n' => self.parse_null(),
                't' => self.parse_bool_true(),
                'f' => self.parse_bool_false(),
                '/' => self.parse_comment(),
                '"' => self.parse_string(),
                c => {
                    if c.is_numeric() || c == '-' {
                        self.parse_number()
                    } else {
                        Some(Err(Error::UnexpectedCharacter((i, c))))
                    }
                }
            }
        } else {
            None
        }
    }

    fn parse_number(&mut self) -> Option<ScanResult<'a>> {
        // TODO(ryanfowler): Parse a number properly.
        let start = self.current_idx + 1;

        match self.next_char() {
            None => {
                return Some(Err(Error::UnexpectedEOF));
            }
            Some((i, c)) => {
                if c != '-' && !c.is_numeric() && c != 'e' && c != 'E' {
                    return Some(Err(Error::UnexpectedCharacter((i, c))));
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

        Some(Ok(Token::Number(&self.input[start..end])))
    }

    fn parse_string(&mut self) -> Option<ScanResult<'a>> {
        match self.next_char() {
            Some((_, '"')) => {}
            Some((i, c)) => return Some(Err(Error::UnexpectedCharacter((i, c)))),
            None => return Some(Err(Error::UnexpectedEOF)),
        }

        let start = self.current_idx + 1;
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
                                            return Some(Err(Error::UnexpectedCharacter((i, c))));
                                        }
                                    }
                                    None => return Some(Err(Error::UnexpectedEOF)),
                                }
                            }
                        }
                        c => return Some(Err(Error::UnexpectedCharacter((i, c)))),
                    },
                    None => return Some(Err(Error::UnexpectedEOF)),
                },
                '"' => {
                    let end = self.current_idx;
                    return Some(Ok(Token::String(&self.input[start..end])));
                }
                _ => {}
            }
        }
        Some(Err(Error::UnexpectedEOF))
    }

    fn parse_comment(&mut self) -> Option<ScanResult<'a>> {
        match self.next_char() {
            Some((_, '/')) => match self.next_char() {
                Some((_, '/')) => self.parse_line_comment(),
                Some((_, '*')) => self.parse_block_comment(),
                Some(v) => Some(Err(Error::UnexpectedCharacter(v))),
                None => Some(Err(Error::UnexpectedEOF)),
            },
            Some(v) => Some(Err(Error::UnexpectedCharacter(v))),
            None => Some(Err(Error::UnexpectedEOF)),
        }
    }

    fn parse_line_comment(&mut self) -> Option<ScanResult<'a>> {
        let start = self.current_idx + 1;
        let mut end = start;
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
        Some(Ok(Token::LineComment(&self.input[start..end])))
    }

    fn parse_block_comment(&mut self) -> Option<ScanResult<'a>> {
        let start = self.current_idx + 1;
        let mut end;
        while let Some((i, c)) = self.next_char() {
            end = i;
            if c == '*' {
                if let Some((_, '/')) = self.peek_char() {
                    self.skip_char();
                    return Some(Ok(Token::BlockComment(&self.input[start..end])));
                }
            }
        }
        Some(Err(Error::UnexpectedEOF))
    }

    fn parse_null(&mut self) -> Option<ScanResult<'a>> {
        let start = self.current_idx;
        if self.next_chars_equal("null") {
            Some(Ok(Token::Null))
        } else {
            Some(Err(Error::UnexpectedCharacter((start, 'n'))))
        }
    }

    fn parse_bool_true(&mut self) -> Option<ScanResult<'a>> {
        let start = self.current_idx;
        if self.next_chars_equal("true") {
            Some(Ok(Token::Bool(true)))
        } else {
            Some(Err(Error::UnexpectedCharacter((start, 't'))))
        }
    }

    fn parse_bool_false(&mut self) -> Option<ScanResult<'a>> {
        let start = self.current_idx;
        if self.next_chars_equal("false") {
            Some(Ok(Token::Bool(false)))
        } else {
            Some(Err(Error::UnexpectedCharacter((start, 'f'))))
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
