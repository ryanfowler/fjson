use std::fmt::{self, Write};
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
    pub typ: ValueType<'a>,
    pub comments: Vec<Comment<'a>>,
}

#[derive(Debug)]
pub enum ValueType<'a> {
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
        value: Value { typ, comments },
        meta_below,
    })
}

fn parse_next_value<'a, I>(s: &mut Peekable<I>) -> Result<ValueType<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    if let Some(event) = next_event(s)? {
        parse_value(s, event)
    } else {
        Err(Error::UnexpectedEOF)
    }
}

fn parse_value<'a, I>(s: &mut Peekable<I>, event: Event<'a>) -> Result<ValueType<'a>, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
{
    let typ = match event.token {
        Token::ObjectStart => parse_object(s)?,
        Token::ArrayStart => parse_array(s)?,
        Token::Null => ValueType::Null,
        Token::String(v) => ValueType::String(v),
        Token::Number(v) => ValueType::Number(v),
        Token::Bool(v) => ValueType::Bool(v),
        _ => return Err(Error::UnexpectedToken(event.into())),
    };
    Ok(typ)
}

fn parse_object<'a, I>(s: &mut Peekable<I>) -> Result<ValueType<'a>, Error>
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

                vals.push(ObjectValue::KeyVal(key, Value { typ, comments }));

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

    Ok(ValueType::Object(vals))
}

fn parse_array<'a, I>(s: &mut Peekable<I>) -> Result<ValueType<'a>, Error>
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

        vals.push(ArrayValue::ArrayVal(Value { typ, comments }));

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

    Ok(ValueType::Array(vals))
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

pub fn format_jsonc(root: Root) -> String {
    let mut out = String::new();
    for meta in root.meta_above {
        _ = write_metadata(&mut out, meta);
        _ = out.write_char('\n');
    }
    _ = write_value(&mut out, root.value.typ, 0, 0);
    _ = write_comments(&mut out, &root.value.comments);
    for meta in root.meta_below {
        _ = write_metadata(&mut out, meta);
        _ = out.write_char('\n');
    }
    out
}

struct Context<'a, W: Write> {
    w: &'a mut W,
    written: usize,
}

impl<'a, W: Write> Context<'a, W> {
    fn write_json_string(&mut self, s: &str) -> Result<(), Error> {
        self.write_char('"')?;
        self.write_str(s)?;
        self.write_char('"')
    }

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_str(s)?;
        self.written += s.len();
        Ok(())
    }

    fn write_newline(&mut self) -> Result<(), Error> {
        self.write_char('\n')?;
        self.written = 0;
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), Error> {
        self.w.write_char(c)?;
        self.written += 1;
        Ok(())
    }
}

const INDENT: &str = "  ";
const LINE_LENGTH: usize = 80;

fn write_value<'a, W: Write>(
    ctx: &mut Context<'a, W>,
    value: ValueType,
    written: usize,
    indent: usize,
) -> Result<usize, Error> {
    match value {
        ValueType::Object(vals) => {
            let mut written = written;
            let length = vals.len();
            let same_line =
                LINE_LENGTH > written && can_fit_object(&vals, LINE_LENGTH - written).is_some();

            ctx.write_char('{')?;
            written += 1;
            for (i, val) in vals.into_iter().enumerate() {
                if same_line {
                    w.write_char(' ')?;
                    written += 1;
                } else {
                    w.write_char('\n')?;
                    written = write_indent(w, indent + 1)?;
                }
                match val {
                    ObjectValue::KeyVal(k, v) => {
                        written += write_string(w, k)?;
                        w.write_str(": ")?;
                        written += 2;
                        written = write_value(w, v.typ, written, indent + 1)?;
                        if i < length - 1 {
                            w.write_char(',')?;
                            written += 1;
                        }
                        written += write_comments(w, &v.comments)?;
                    }
                    ObjectValue::Metadata(meta) => written += write_metadata(w, meta)?,
                }
            }
            if length > 0 {
                if same_line {
                    w.write_char(' ')?;
                    written += 1;
                } else {
                    w.write_char('\n')?;
                    written = write_indent(w, indent)?;
                }
            }
            w.write_char('}')?;
            written += 1;
            Ok(written)
        }
        ValueType::Array(vals) => {
            let mut written = written;
            let length = vals.len();
            let same_line =
                LINE_LENGTH > written && can_fit_array(&vals, LINE_LENGTH - written).is_some();

            w.write_char('[')?;
            written += 1;
            for (i, val) in vals.into_iter().enumerate() {
                if same_line {
                    if i > 0 {
                        w.write_char(' ')?;
                        written += 1;
                    }
                } else {
                    w.write_char('\n')?;
                    written = write_indent(w, indent + 1)?;
                }
                match val {
                    ArrayValue::ArrayVal(v) => {
                        written = write_value(w, v.typ, written, indent + 1)?;
                        if i < length - 1 {
                            w.write_char(',')?;
                            written += 1;
                        }
                        written += write_comments(w, &v.comments)?;
                    }
                    ArrayValue::Metadata(meta) => written += write_metadata(w, meta)?,
                }
            }
            if length > 0 && !same_line {
                w.write_char('\n')?;
                written = write_indent(w, indent)?;
            }
            w.write_char(']')?;
            written += 1;
            Ok(written)
        }
        ValueType::String(v) => Ok(written + write_string(w, v)?),
        ValueType::Number(v) => {
            w.write_str(v)?;
            Ok(written + v.len())
        }
        ValueType::Bool(v) => {
            if v {
                w.write_str("true")?;
                Ok(written + 4)
            } else {
                w.write_str("false")?;
                Ok(written + 5)
            }
        }
        ValueType::Null => {
            w.write_str("null")?;
            Ok(written + 4)
        }
    }
}

fn write_string<W: fmt::Write>(w: &mut W, v: &str) -> Result<usize, Error> {
    w.write_char('"')?;
    w.write_str(v)?;
    w.write_char('"')?;
    Ok(2 + v.len())
}

fn can_fit_value(val: &ValueType, space: usize) -> Option<usize> {
    let remaining = space as i64;
    let remaining = match val {
        ValueType::Object(v) => return can_fit_object(v, space),
        ValueType::Array(v) => return can_fit_array(v, space),
        ValueType::String(v) => remaining - (2 + v.len() as i64),
        ValueType::Number(v) => remaining - v.len() as i64,
        ValueType::Bool(v) => {
            if *v {
                remaining - 4
            } else {
                remaining - 5
            }
        }
        ValueType::Null => remaining - 4,
    };
    if remaining >= 0 {
        Some(remaining as usize)
    } else {
        None
    }
}

fn can_fit_object(vals: &Vec<ObjectValue>, space: usize) -> Option<usize> {
    let num_vals = vals.len() as i64;
    let mut remaining = (space as i64) - 2; // For object start/close.
    if !vals.is_empty() {
        // Object padding + (key quotes + colon + padding) * values + (comma + padding) * values - 1.
        remaining -= 2 + 4 * num_vals + 2 * (num_vals - 1);
    }
    if remaining < 0 {
        return None;
    }
    for val in vals {
        match val {
            ObjectValue::Metadata(_) => return None,
            ObjectValue::KeyVal(k, v) => {
                if !v.comments.is_empty() {
                    return None;
                }
                remaining -= k.len() as i64;
                if remaining < 0 {
                    return None;
                }
                match can_fit_value(&v.typ, remaining as usize) {
                    None => return None,
                    Some(size) => {
                        remaining = size as i64;
                    }
                }
            }
        }
    }

    if remaining >= 0 {
        Some(remaining as usize)
    } else {
        None
    }
}

fn can_fit_array(vals: &Vec<ArrayValue>, space: usize) -> Option<usize> {
    let num_vals = vals.len() as i64;
    let mut remaining = (space as i64) - 2; // For array start/close.
    if !vals.is_empty() {
        // (comma + padding) * values - 1.
        remaining -= 2 * (num_vals - 1);
    }
    if remaining < 0 {
        return None;
    }
    for val in vals {
        match val {
            ArrayValue::Metadata(_) => return None,
            ArrayValue::ArrayVal(v) => {
                if !v.comments.is_empty() {
                    return None;
                }
                match can_fit_value(&v.typ, remaining as usize) {
                    None => return None,
                    Some(size) => {
                        remaining = size as i64;
                    }
                }
            }
        }
    }

    if remaining >= 0 {
        Some(remaining as usize)
    } else {
        None
    }
}

fn write_metadata<W: fmt::Write>(w: &mut W, meta: Metadata) -> Result<usize, Error> {
    Ok(match meta {
        Metadata::Newline => 0,
        Metadata::Comment(c) => write_comment(w, &c)?,
    })
}

fn write_comments<W: fmt::Write>(w: &mut W, cs: &[Comment]) -> Result<usize, Error> {
    if cs.is_empty() {
        return Ok(0);
    }
    w.write_char(' ')?;
    let mut written = 1;
    for comment in cs {
        written += write_comment(w, comment)?;
    }
    Ok(written)
}

fn write_comment<W: fmt::Write>(w: &mut W, comment: &Comment) -> Result<usize, Error> {
    match comment {
        Comment::Block(c) => {
            w.write_str("/*")?;
            w.write_str(c)?;
            w.write_str("*/")?;
            Ok(4 + c.len())
        }
        Comment::Line(c) => {
            w.write_str("//")?;
            w.write_str(c)?;
            Ok(2 + c.len())
        }
    }
}

fn write_indent<W: fmt::Write>(w: &mut W, n: usize) -> Result<usize, Error> {
    for _ in 0..n {
        w.write_str(INDENT)?;
    }
    Ok(n * INDENT.len())
}

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
        let out = format_jsonc(root);
        println!("{}", out);
    }
}
