//! Error handling for the fjson crate.

use std::{
    error,
    fmt::{self, Display},
    ops::Range,
};

use crate::scanner::{Event, Token};

/// The error type used in this crate.
#[derive(Clone, Debug)]
pub enum Error {
    /// The maximum allowed recursion was exceeded.
    RecursionLimitExceeded,
    /// An unexpected character was encountered when tokenizing the JSON source.
    UnexpectedCharacter(usize, char),
    /// An unexpected JSON token was encountered when parsing the source.
    UnexpectedToken(Range<usize>, TokenType),
    /// The end-of-file was reached while parsing the JSON source.
    UnexpectedEOF,
    /// Error formatting the JSON to the std::fmt::Writer provided.
    Write(fmt::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        if let Error::Write(err) = self {
            Some(err)
        } else {
            None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursionLimitExceeded => write!(f, "maximum recursion limit exceeded"),
            Self::UnexpectedCharacter(i, c) => {
                write!(f, "unexpected character at index {i}: '{c}'")
            }
            Self::UnexpectedToken(range, typ) => {
                write!(
                    f,
                    "unexpected token at index range {} -> {}: '{}'",
                    range.start, range.end, typ
                )
            }
            Self::UnexpectedEOF => f.write_str("unexpected end of file"),
            Self::Write(err) => write!(f, "writing: {err}"),
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        Error::Write(value)
    }
}

impl std::convert::From<Event<'_>> for Error {
    fn from(value: Event<'_>) -> Self {
        Error::UnexpectedToken(value.range, TokenType::from(value.token))
    }
}

impl std::convert::From<&Event<'_>> for Error {
    fn from(value: &Event<'_>) -> Self {
        Error::UnexpectedToken(value.range.clone(), TokenType::from(value.token))
    }
}

/// The different types of JSON tokens.
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
