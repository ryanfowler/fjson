use std::iter::Peekable;

use crate::error::Error;
use crate::scanner::{Event, ScanResult, Scanner, Token};

#[derive(Debug)]
pub struct Root<'a> {
    pub meta_above: Vec<Metadata<'a>>,
    pub value: Value<'a>,
    pub meta_below: Vec<Metadata<'a>>,
}

#[derive(Debug)]
pub struct Value<'a> {
    pub token: ValueToken<'a>,
    pub comments: Vec<Comment<'a>>,
}

#[derive(Debug)]
pub enum ValueToken<'a> {
    Object(Vec<ObjectValue<'a>>),
    Array(Vec<ArrayValue<'a>>),
    String(&'a str),
    Number(&'a str),
    Bool(bool),
    Null,
}

#[derive(Debug)]
pub enum ArrayValue<'a> {
    Metadata(Metadata<'a>),
    ArrayVal(Value<'a>),
}

#[derive(Debug)]
pub enum ObjectValue<'a> {
    Metadata(Metadata<'a>),
    KeyVal(&'a str, Value<'a>),
}

#[derive(Debug)]
pub enum Metadata<'a> {
    Comment(Comment<'a>),
    Newline,
}

#[derive(Debug)]
pub enum Comment<'a> {
    Line(&'a str),
    Block(&'a str),
}

pub fn parse(input: &str) -> Result<Root, Error> {
    let mut s = Scanner::new(input).peekable();
    parse_newlines(&mut s)?;
    let mut meta_above = Vec::new();
    while let Some(meta) = parse_metadata(&mut s)? {
        meta_above.push(meta);
    }
    let typ = parse_next_value(&mut s)?;
    let comments = parse_sameline_comments(&mut s)?;
    let mut meta_below = Vec::new();
    while let Some(meta) = parse_metadata(&mut s)? {
        meta_below.push(meta);
    }
    if let Some(Metadata::Newline) = meta_below.last() {
        meta_below.pop();
    }
    Ok(Root {
        meta_above,
        value: Value {
            token: typ,
            comments,
        },
        meta_below,
    })
}

fn parse_next_value<'a, I>(s: &mut Peekable<I>) -> Result<ValueToken<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    if let Some(event) = next_event(s)? {
        parse_value(s, event)
    } else {
        Err(Error::UnexpectedEOF)
    }
}

fn parse_value<'a, I>(s: &mut Peekable<I>, event: Event<'a>) -> Result<ValueToken<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    let typ = match event.token {
        Token::ObjectStart => parse_object(s)?,
        Token::ArrayStart => parse_array(s)?,
        Token::Null => ValueToken::Null,
        Token::String(v) => ValueToken::String(v),
        Token::Number(v) => ValueToken::Number(v),
        Token::Bool(v) => ValueToken::Bool(v),
        _ => return Err(Error::UnexpectedToken(event.into())),
    };
    Ok(typ)
}

fn parse_object<'a, I>(s: &mut Peekable<I>) -> Result<ValueToken<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    skip_newlines(s)?;

    let mut vals = Vec::new();
    loop {
        while let Some(meta) = parse_metadata(s)? {
            vals.push(ObjectValue::Metadata(meta));
        }

        let event = match next_event(s)? {
            Some(event) => event,
            None => return Err(Error::UnexpectedEOF),
        };
        match event.token {
            Token::ObjectEnd => break,
            Token::String(key) => {
                skip_newlines(s)?;
                while let Some(meta) = parse_metadata(s)? {
                    vals.push(ObjectValue::Metadata(meta));
                }

                match next_event(s)? {
                    Some(Event {
                        token: Token::Colon,
                        range: _,
                    }) => {}
                    Some(event) => return Err(Error::UnexpectedToken(event.into())),
                    None => return Err(Error::UnexpectedEOF),
                }

                skip_newlines(s)?;
                while let Some(meta) = parse_metadata(s)? {
                    vals.push(ObjectValue::Metadata(meta));
                }

                let typ = parse_next_value(s)?;
                let mut comments = Vec::new();

                let mut comma = false;
                while let Some(event) = peek_event(s)? {
                    match event.token {
                        Token::Newline => {
                            break;
                        }
                        Token::Comma => {
                            if comma {
                                return Err(Error::UnexpectedToken(event.into()));
                            }
                            skip_event(s)?;
                            comma = true;
                        }
                        Token::LineComment(c) => {
                            skip_event(s)?;
                            comments.push(Comment::Line(c));
                        }
                        Token::BlockComment(c) => {
                            skip_event(s)?;
                            comments.push(Comment::Block(c));
                        }
                        _ => break,
                    }
                }

                vals.push(ObjectValue::KeyVal(
                    key,
                    Value {
                        token: typ,
                        comments,
                    },
                ));

                if !comma {
                    while let Some(meta) = parse_metadata(s)? {
                        vals.push(ObjectValue::Metadata(meta));
                    }
                    match next_event(s)? {
                        None => return Err(Error::UnexpectedEOF),
                        Some(event) => match event.token {
                            Token::Comma => {}
                            Token::ObjectEnd => break,
                            _ => return Err(Error::UnexpectedToken(event.into())),
                        },
                    }
                }
            }
            _ => return Err(Error::UnexpectedToken(event.into())),
        }
    }

    while let Some(ObjectValue::Metadata(Metadata::Newline)) = vals.last() {
        vals.pop();
    }

    Ok(ValueToken::Object(vals))
}

fn parse_array<'a, I>(s: &mut Peekable<I>) -> Result<ValueToken<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    skip_newlines(s)?;

    let mut vals = Vec::new();
    loop {
        while let Some(meta) = parse_metadata(s)? {
            vals.push(ArrayValue::Metadata(meta));
        }

        if let Some(event) = peek_event(s)? {
            if event.token == Token::ArrayEnd {
                skip_event(s)?;
                break;
            }
        }

        let typ = parse_next_value(s)?;
        let mut comments = Vec::new();

        let mut comma = false;
        while let Some(event) = peek_event(s)? {
            match event.token {
                Token::Newline => {
                    break;
                }
                Token::Comma => {
                    if comma {
                        return Err(Error::UnexpectedToken(event.into()));
                    }
                    skip_event(s)?;
                    comma = true;
                }
                Token::LineComment(c) => {
                    skip_event(s)?;
                    comments.push(Comment::Line(c));
                }
                Token::BlockComment(c) => {
                    skip_event(s)?;
                    comments.push(Comment::Block(c));
                }
                _ => break,
            }
        }

        vals.push(ArrayValue::ArrayVal(Value {
            token: typ,
            comments,
        }));

        if !comma {
            while let Some(meta) = parse_metadata(s)? {
                vals.push(ArrayValue::Metadata(meta));
            }
            match next_event(s)? {
                None => return Err(Error::UnexpectedEOF),
                Some(event) => match event.token {
                    Token::Comma => {}
                    Token::ArrayEnd => break,
                    _ => return Err(Error::UnexpectedToken(event.into())),
                },
            }
        }
    }

    while let Some(ArrayValue::Metadata(Metadata::Newline)) = vals.last() {
        vals.pop();
    }

    Ok(ValueToken::Array(vals))
}

fn parse_newlines<'a, I>(s: &mut Peekable<I>) -> Result<usize, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    let mut newlines = 0;
    while let Some(event) = peek_event(s)? {
        match event.token {
            Token::Newline => {
                skip_event(s)?;
                newlines += 1;
            }
            _ => break,
        }
    }
    Ok(newlines)
}

fn parse_sameline_comments<'a, I>(s: &mut Peekable<I>) -> Result<Vec<Comment<'a>>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    let mut out = Vec::new();
    while let Some(event) = peek_event(s)? {
        match event.token {
            Token::LineComment(c) => {
                skip_event(s)?;
                out.push(Comment::Line(c));
            }
            Token::BlockComment(c) => {
                skip_event(s)?;
                out.push(Comment::Block(c));
            }
            _ => break,
        }
    }
    Ok(out)
}

fn parse_metadata<'a, I>(s: &mut Peekable<I>) -> Result<Option<Metadata<'a>>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    while let Some(event) = peek_event(s)? {
        match event.token {
            Token::LineComment(c) => {
                skip_event(s)?;
                return Ok(Some(Metadata::Comment(Comment::Line(c))));
            }
            Token::BlockComment(c) => {
                skip_event(s)?;
                return Ok(Some(Metadata::Comment(Comment::Block(c))));
            }
            Token::Newline => {
                skip_event(s)?;
                if parse_newlines(s)? > 0 {
                    return Ok(Some(Metadata::Newline));
                }
            }
            _ => break,
        }
    }
    Ok(None)
}

fn skip_event<'a, I>(s: &mut Peekable<I>) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    next_event(s)?;
    Ok(())
}

fn next_event<'a, I>(s: &mut Peekable<I>) -> Result<Option<Event<'a>>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    match s.next() {
        Some(Ok(event)) => Ok(Some(event)),
        Some(Err(err)) => Err(err),
        None => Ok(None),
    }
}

fn peek_event<'a, I>(s: &mut Peekable<I>) -> Result<Option<&Event<'a>>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    match s.peek() {
        Some(Ok(event)) => Ok(Some(event)),
        None => Ok(None),
        Some(Err(err)) => Err(err.clone()),
    }
}

fn skip_newlines<'a, I>(s: &mut Peekable<I>) -> Result<usize, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    let mut newlines = 0;
    while let Some(event) = peek_event(s)? {
        if event.token != Token::Newline {
            break;
        }
        newlines += 1;
        skip_event(s)?;
    }
    Ok(newlines)
}

pub fn strip_metadata(root: &mut Root) {
    root.meta_above.clear();
    root.meta_below.clear();
    strip_meta_value(&mut root.value);
}

fn strip_meta_value(value: &mut Value) {
    value.comments.clear();
    match value.token {
        ValueToken::Object(ref mut vals) => vals.retain_mut(|v| match v {
            ObjectValue::Metadata(_) => false,
            ObjectValue::KeyVal(_, ref mut v) => {
                strip_meta_value(v);
                true
            }
        }),
        ValueToken::Array(ref mut vals) => vals.retain_mut(|v| match v {
            ArrayValue::Metadata(_) => false,
            ArrayValue::ArrayVal(ref mut v) => {
                strip_meta_value(v);
                true
            }
        }),
        _ => {}
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_jsonc() {
        let input = r#"
        // This is a comment.
        // Second line.

        // Break, than third.

        { // Object start.

            "key1": "val1", // Same line comment.
            "k": "v",
            // Next line comment.
            "arr_key": [ // Array start.

                "val1"
                ,
                100 // Before comma
                ,

                // True.
                true,
            ],

            // And another.
        "key2": { "nested": // And another one.
        100, "value": true, "third": "this"

        // Weird comment before comma.
        , "is": "a", "v":{"another" :"object",}, "a": ["value", "this value should cause the array to go multi-line, I think."]  },
        } // Trailing comment."#;
        println!("{}", input);
        let root = parse(input).unwrap();
        println!("{:#?}", root);
        let mut out = String::new();
        //format_jsonc(&mut out, root).unwrap();
        println!("{}", out);
    }
}
*/
