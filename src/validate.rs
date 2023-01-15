use std::iter::Peekable;

use crate::{
    scanner::{Event, ScanResult, Token},
    Error,
};

use arrayvec::ArrayVec;

const MAX_RECURSION: usize = 129; // 128 + 1 for the root value itself.

/// Trait that can be used to validate an `Iterator` of [ScanResult]s.
pub trait ValidateIter<'a>: Iterator<Item = ScanResult<'a>> {
    fn validate(self) -> Validate<'a, Self>
    where
        Self: std::marker::Sized,
    {
        Validate::new(self)
    }
}

impl<'a, I: Iterator<Item = ScanResult<'a>>> ValidateIter<'a> for I {}

#[derive(Debug)]
enum State {
    Array(ArrayState),
    Object(ObjectState),
    Value,
}

#[derive(Debug)]
enum ArrayState {
    Start,
    Value,
    Comma,
}

#[derive(Debug)]
enum ObjectState {
    Start,
    Key,
    Colon,
    Value,
    Comma,
}

/// Validate an `Iterator` of [ScanResult]s without building an AST
/// [crate::ast::Root] struct.
pub struct Validate<'a, I: Iterator<Item = ScanResult<'a>>> {
    iter: Peekable<I>,
    has_error: bool,
    stack: ArrayVec<State, MAX_RECURSION>,
}

impl<'a, I> Iterator for Validate<'a, I>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    type Item = ScanResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_error {
            return None;
        }
        match self.next_option() {
            Some(Ok(event)) => Some(Ok(event)),
            Some(Err(err)) => {
                self.has_error = true;
                Some(Err(err))
            }
            None => None,
        }
    }
}

impl<'a, I> Validate<'a, I>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    /// Wrap the provided `Iterator` to ensure that the input is valid JSON(C).
    pub fn new(iter: I) -> Self {
        Self {
            iter: iter.peekable(),
            has_error: false,
            stack: ArrayVec::new(),
        }
    }

    fn next_option(&mut self) -> Option<ScanResult<'a>> {
        match self.get_next() {
            Ok(Some(res)) => Some(Ok(res)),
            Ok(None) => match self.stack.pop() {
                Some(State::Value) => {
                    if self.stack.is_empty() {
                        None
                    } else {
                        Some(Err(Error::UnexpectedEOF))
                    }
                }
                _ => Some(Err(Error::UnexpectedEOF)),
            },
            Err(err) => {
                self.has_error = true;
                Some(Err(err))
            }
        }
    }

    fn get_next(&mut self) -> Result<Option<Event<'a>>, Error> {
        if let Some(event) = self.next_event()? {
            match event.token {
                Token::ObjectStart => {
                    let state = match self.stack.last() {
                        Some(State::Array(ArrayState::Start | ArrayState::Comma)) => {
                            State::Array(ArrayState::Value)
                        }
                        Some(State::Object(ObjectState::Colon)) => {
                            State::Object(ObjectState::Value)
                        }
                        None => State::Value,
                        _ => return Err(event.into()),
                    };
                    self.set_last_state(state);
                    self.push_to_stack(State::Object(ObjectState::Start))?;
                }
                Token::ObjectEnd => {
                    if !matches!(
                        self.stack.last(),
                        Some(
                            State::Object(ObjectState::Start)
                                | State::Object(ObjectState::Value)
                                | State::Object(ObjectState::Comma)
                        )
                    ) {
                        return Err(event.into());
                    }
                    self.stack.pop();
                }
                Token::ArrayStart => {
                    let state = match self.stack.last() {
                        Some(State::Array(ArrayState::Start | ArrayState::Comma)) => {
                            State::Array(ArrayState::Value)
                        }
                        Some(State::Object(ObjectState::Colon)) => {
                            State::Object(ObjectState::Value)
                        }
                        None => State::Value,
                        _ => return Err(event.into()),
                    };
                    self.set_last_state(state);
                    self.push_to_stack(State::Array(ArrayState::Start))?;
                }
                Token::ArrayEnd => {
                    if !matches!(
                        self.stack.last(),
                        Some(
                            State::Array(ArrayState::Start)
                                | State::Array(ArrayState::Value)
                                | State::Array(ArrayState::Comma)
                        )
                    ) {
                        return Err(event.into());
                    }
                    self.stack.pop();
                }
                Token::Comma => {
                    let next = match self.stack.last() {
                        Some(State::Object(ObjectState::Value)) => {
                            State::Object(ObjectState::Comma)
                        }
                        Some(State::Array(ArrayState::Value)) => State::Array(ArrayState::Comma),
                        _ => return Err(event.into()),
                    };
                    self.set_last_state(next);
                    if let Some(event) = self.peek_next()? {
                        if matches!(event.token, Token::ArrayEnd | Token::ObjectEnd) {
                            return self.get_next();
                        }
                    }
                }
                Token::Colon => match self.stack.last_mut() {
                    Some(state) => match state {
                        State::Object(ObjectState::Key) => {
                            *state = State::Object(ObjectState::Colon)
                        }
                        _ => return Err(event.into()),
                    },
                    _ => return Err(event.into()),
                },
                Token::Null | Token::Number(_) | Token::Bool(_) => self.handle_value(&event)?,
                Token::String(_) => match self.stack.last() {
                    Some(State::Object(ObjectState::Start | ObjectState::Comma)) => {
                        self.set_last_state(State::Object(ObjectState::Key));
                    }
                    _ => self.handle_value(&event)?,
                },
                _ => {}
            }
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    fn handle_value(&mut self, event: &Event) -> Result<(), Error> {
        match self.stack.last_mut() {
            Some(state) => match state {
                State::Array(ArrayState::Start | ArrayState::Comma) => {
                    *state = State::Array(ArrayState::Value);
                }
                State::Object(ObjectState::Colon) => *state = State::Object(ObjectState::Value),
                _ => return Err(event.into()),
            },
            None => self.push_to_stack(State::Value)?,
        }
        Ok(())
    }

    fn push_to_stack(&mut self, typ: State) -> Result<(), Error> {
        if self.stack.try_push(typ).is_ok() {
            Ok(())
        } else {
            Err(Error::RecursionLimitExceeded)
        }
    }

    fn set_last_state(&mut self, typ: State) {
        if let Some(state) = self.stack.last_mut() {
            *state = typ;
        } else {
            self.stack.push(typ);
        }
    }

    fn peek_next(&mut self) -> Result<Option<Event<'a>>, Error> {
        loop {
            let event = match self.iter.peek() {
                Some(result) => match result {
                    Ok(event) => event,
                    Err(err) => return Err(err.clone()),
                },
                None => return Ok(None),
            };
            if !matches!(
                event.token,
                Token::LineComment(_) | Token::BlockComment(_) | Token::Newline
            ) {
                return Ok(Some(event.clone()));
            }
            self.next_event()?;
        }
    }

    fn next_event(&mut self) -> Result<Option<Event<'a>>, Error> {
        match self.iter.next() {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(err)) => Err(err),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{Event, Scanner, Token};

    #[test]
    fn test_validate() {
        let input = r#"{"key":true}"#;
        let expected = vec![
            Event {
                token: Token::ObjectStart,
                range: 0..1,
            },
            Event {
                token: Token::String("key"),
                range: 1..6,
            },
            Event {
                token: Token::Colon,
                range: 6..7,
            },
            Event {
                token: Token::Bool(true),
                range: 7..11,
            },
            Event {
                token: Token::ObjectEnd,
                range: 11..12,
            },
        ];

        let iter = Validate::new(Scanner::new(input));
        let out = iter.map(|v| v.unwrap()).collect::<Vec<_>>();
        assert_eq!(out, expected);
    }

    #[test]
    fn test_validate_fail() {
        let input = r#"{"key":true"#;
        let iter = Validate::new(Scanner::new(input));
        let result: Result<Vec<_>, _> = iter.collect();
        assert_eq!(result, Err(crate::Error::UnexpectedEOF));
    }
}
