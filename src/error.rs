use std::{
    fmt::{self, Display},
    ops::Range,
};

use crate::scanner::{Event, Token};

#[derive(Clone, Debug)]
pub enum Error {
    UnexpectedCharacter((usize, char)),
    UnexpectedToken((Range<usize>, TokenType)),
    UnexpectedEOF,
    Write(fmt::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter((i, c)) => {
                write!(f, "unexpected character at index {}: '{}'", i, c)
            }
            Self::UnexpectedToken((range, typ)) => {
                write!(
                    f,
                    "unexpected token at index range {} -> {}: '{}'",
                    range.start, range.end, typ
                )
            }
            Self::UnexpectedEOF => f.write_str("unexpected end of file"),
            Self::Write(err) => write!(f, "writing: {}", err),
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        Error::Write(value)
    }
}

impl std::convert::From<Event<'_>> for (Range<usize>, TokenType) {
    fn from(value: Event<'_>) -> Self {
        (value.range, TokenType::from(value.token))
    }
}

impl std::convert::From<&Event<'_>> for (Range<usize>, TokenType) {
    fn from(value: &Event<'_>) -> Self {
        (value.range.clone(), TokenType::from(value.token))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TokenType {
    Newline,
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    Comma,
    Colon,
    Null,
    LineComment,
    BlockComment,
    String,
    Number,
    Bool,
}

impl std::convert::From<Token<'_>> for TokenType {
    fn from(value: Token<'_>) -> Self {
        match value {
            Token::Newline => TokenType::Newline,
            Token::ObjectStart => TokenType::ObjectStart,
            Token::ObjectEnd => TokenType::ObjectEnd,
            Token::ArrayStart => TokenType::ArrayStart,
            Token::ArrayEnd => TokenType::ArrayEnd,
            Token::Comma => TokenType::Comma,
            Token::Colon => TokenType::Colon,
            Token::Null => TokenType::Null,
            Token::LineComment(_) => TokenType::LineComment,
            Token::BlockComment(_) => TokenType::BlockComment,
            Token::String(_) => TokenType::String,
            Token::Number(_) => TokenType::Number,
            Token::Bool(_) => TokenType::Bool,
        }
    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            TokenType::Newline => "\\n",
            TokenType::ObjectStart => "{",
            TokenType::ObjectEnd => "}",
            TokenType::ArrayStart => "[",
            TokenType::ArrayEnd => "]",
            TokenType::Comma => ",",
            TokenType::Colon => ":",
            TokenType::Null => "null",
            TokenType::LineComment => "line comment",
            TokenType::BlockComment => "block comment",
            TokenType::String => "string",
            TokenType::Number => "number",
            TokenType::Bool => "bool",
        };
        f.write_str(out)
    }
}
