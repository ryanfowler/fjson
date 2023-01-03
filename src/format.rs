use std::{fmt::Write, iter::Peekable};

use crate::{
    error::Error,
    scanner::{Event, ScanResult, Token},
};

pub fn format_json<'a, I, W>(s: I, w: &mut W) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    let iter = s.filter(|res| {
        if let Ok(event) = res {
            match event.token {
                Token::LineComment(_) | Token::BlockComment(_) | Token::Newline => return false,
                _ => {}
            }
        }
        true
    });
    format_jsonc(iter, w)
}

pub fn format_jsonc<'a, I, W>(s: I, w: &mut W) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    let mut s = s.peekable();
    skip_newlines(&mut s)?;
    if format_comments(&mut s, w, 0, CommentStart::None)? > 0 {
        write_char(w, '\n')?;
    }
    format_value(&mut s, w, 0)?;
    format_comments(&mut s, w, 0, CommentStart::Space)?;
    if let Some(event) = next_event(&mut s)? {
        return Err(Error::UnexpectedToken(event.into()));
    }
    write_char(w, '\n')?;
    Ok(())
}

fn format_value<'a, W, I>(s: &mut Peekable<I>, w: &mut W, indent: usize) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    format_comments(s, w, indent, CommentStart::Newline)?;
    if let Some(event) = next_event(s)? {
        match event.token {
            Token::ObjectStart => format_object(s, w, indent),
            Token::ArrayStart => format_array(s, w, indent),
            Token::Bool(v) => format_bool(w, v),
            Token::Null => format_null(w),
            Token::Number(v) => format_number(w, v),
            Token::String(v) => format_string(w, v),
            _ => Err(Error::UnexpectedToken(event.into())),
        }
    } else {
        Err(Error::UnexpectedEOF)
    }
}

enum CommentStart {
    None,
    Space,
    Newline,
}

fn format_comments<'a, W, I>(
    s: &mut Peekable<I>,
    w: &mut W,
    indent: usize,
    start: CommentStart,
) -> Result<usize, Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    let mut comments_written = 0;
    if let Some(event) = peek_event(s)? {
        if let Token::LineComment(v) = event.token {
            match start {
                CommentStart::None => {}
                CommentStart::Space => write_char(w, ' ')?,
                CommentStart::Newline => {
                    write_char(w, '\n')?;
                    write_indent(w, indent)?;
                }
            }
            skip_event(s)?;
            write_str(w, "//")?;
            write_str(w, v)?;
            comments_written += 1;
        }
    }

    let mut newlines = 0;
    while let Some(event) = peek_event(s)? {
        match event.token {
            Token::LineComment(v) => {
                if newlines > 0 {
                    write_char(w, '\n')?;
                    if newlines > 1 {
                        write_char(w, '\n')?;
                    }
                    write_indent(w, indent)?;
                }
                write_str(w, "//")?;
                write_str(w, v)?;
                newlines = 0;
                comments_written += 1;
            }
            Token::BlockComment(_) => todo!(),
            Token::Newline => newlines += 1,
            _ => {
                // Currently we allow blank lines between fields, but we could
                // change this logic so that blank lines are only allowed after
                // comments by adding the following: "&& comments_written > 0".
                if newlines > 1 {
                    write_char(w, '\n')?;
                }
                break;
            }
        }
        skip_event(s)?;
    }
    Ok(comments_written)
}

fn format_array<'a, W, I>(s: &mut Peekable<I>, w: &mut W, indent: usize) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    write_str(w, "[")?;
    skip_newlines(s)?;

    let mut cs = String::new();
    let mut cnt = 0;
    loop {
        format_comments(s, w, indent + 1, CommentStart::Newline)?;

        if let Some(event) = peek_event(s)? {
            if event.token == Token::ArrayEnd {
                skip_event(s)?;
                break;
            }
        }
        cnt += 1;

        w.write_char('\n')?;
        write_indent(w, indent + 1)?;
        format_value(s, w, indent + 1)?;

        cs.truncate(0);
        format_comments(s, &mut cs, indent + 1, CommentStart::Space)?;

        match next_event(s)? {
            Some(event) => match event.token {
                Token::Comma => {
                    format_comments(s, &mut cs, indent + 1, CommentStart::Space)?;
                    match peek_event(s)? {
                        Some(event) => {
                            if event.token != Token::ArrayEnd {
                                write_char(w, ',')?;
                            }
                        }
                        None => return Err(Error::UnexpectedEOF),
                    }
                }
                Token::ArrayEnd => break,
                _ => return Err(Error::UnexpectedToken(event.into())),
            },
            None => return Err(Error::UnexpectedEOF),
        }

        if !cs.is_empty() {
            write_str(w, &cs)?;
        }
    }
    if cnt > 0 {
        write_char(w, '\n')?;
        write_indent(w, indent)?;
    }
    write_char(w, ']')
}

fn format_object<'a, W, I>(s: &mut Peekable<I>, w: &mut W, indent: usize) -> Result<(), Error>
where
    I: Iterator<Item = ScanResult<'a>>,
    W: Write,
{
    write_str(w, "{")?;
    skip_newlines(s)?;
    format_comments(s, w, indent + 1, CommentStart::Newline)?;

    let mut cs = String::new();
    let mut cnt = 0;
    loop {
        if let Some(event) = next_event(s)? {
            match event.token {
                Token::ObjectEnd => break,
                Token::String(k) => {
                    cnt += 1;
                    skip_newlines(s)?;
                    format_comments(s, w, indent + 1, CommentStart::Newline)?;

                    if let Some(event) = next_event(s)? {
                        match event.token {
                            Token::Colon => {}
                            _ => return Err(Error::UnexpectedToken(event.into())),
                        }
                    } else {
                        return Err(Error::UnexpectedEOF);
                    }

                    skip_newlines(s)?;
                    format_comments(s, w, indent + 1, CommentStart::Newline)?;

                    w.write_char('\n')?;
                    write_indent(w, indent + 1)?;
                    write_char(w, '"')?;
                    write_str(w, k)?;
                    write_str(w, "\": ")?;

                    format_value(s, w, indent + 1)?;

                    cs.truncate(0);
                    format_comments(s, &mut cs, indent + 1, CommentStart::Space)?;

                    if let Some(event) = next_event(s)? {
                        match event.token {
                            Token::Comma => {
                                format_comments(s, &mut cs, indent + 1, CommentStart::Space)?;
                                if let Some(event) = peek_event(s)? {
                                    if event.token != Token::ObjectEnd {
                                        write_char(w, ',')?;
                                    }
                                } else {
                                    return Err(Error::UnexpectedEOF);
                                }
                            }
                            Token::ObjectEnd => break,
                            _ => return Err(Error::UnexpectedToken(event.into())),
                        }
                    } else {
                        return Err(Error::UnexpectedEOF);
                    }

                    if !cs.is_empty() {
                        write_str(w, &cs)?;
                    }
                }
                _ => return Err(Error::UnexpectedToken(event.into())),
            }
        } else {
            return Err(Error::UnexpectedEOF);
        }
    }
    if cnt > 0 {
        write_char(w, '\n')?;
        write_indent(w, indent)?;
    }
    write_char(w, '}')
}

fn format_string<W>(w: &mut W, v: &str) -> Result<(), Error>
where
    W: Write,
{
    w.write_char('"')?;
    w.write_str(v)?;
    w.write_char('"')?;
    Ok(())
}

fn format_number<W>(w: &mut W, v: &str) -> Result<(), Error>
where
    W: Write,
{
    w.write_str(v)?;
    Ok(())
}

fn format_bool<W>(w: &mut W, v: bool) -> Result<(), Error>
where
    W: Write,
{
    let s = if v { "true" } else { "false" };
    w.write_str(s)?;
    Ok(())
}

fn format_null<W>(w: &mut W) -> Result<(), Error>
where
    W: Write,
{
    w.write_str("null")?;
    Ok(())
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

fn write_str<W>(w: &mut W, s: &str) -> Result<(), Error>
where
    W: Write,
{
    w.write_str(s)?;
    Ok(())
}

fn write_char<W>(w: &mut W, c: char) -> Result<(), Error>
where
    W: Write,
{
    w.write_char(c)?;
    Ok(())
}

fn write_indent<W>(w: &mut W, indent: usize) -> Result<(), Error>
where
    W: Write,
{
    for _ in 0..indent {
        w.write_str("  ")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::{read_dir, read_to_string};

    use super::*;
    use crate::scanner::Scanner;

    #[test]
    fn format_success() -> Result<(), Error> {
        let dir = read_dir("./src/tests/format").unwrap();
        for file in dir {
            let file = file.unwrap();
            let file_name = file.file_name();
            let file_str = file_name.to_str().unwrap();
            if file_str.ends_with("-out.jsonc") || file_str.ends_with("-out.json") {
                continue;
            }
            let input = read_to_string(file.path()).unwrap();
            let output = read_to_string(
                file.path()
                    .to_str()
                    .unwrap()
                    .replace(".jsonc", "-out.jsonc"),
            )
            .unwrap();
            let mut out = String::new();
            format_jsonc(Scanner::new(&input), &mut out)?;
            if out != output {
                println!("Got:\n{}-----\nExpected:\n{}-----", out, output);
                panic!("Test failed: {:?}", file.file_name());
            }

            let mut out2 = String::new();
            format_jsonc(Scanner::new(&out), &mut out2)?;
            if out2 != out {
                println!("Got:\n{}-----\nExpected:\n{}-----", out2, out);
                panic!("Test failed: {:?}", file.file_name());
            }

            let mut out3 = String::new();
            format_json(Scanner::new(&input), &mut out3)?;
            let json_val =
                read_to_string(file.path().to_str().unwrap().replace(".jsonc", "-out.json"))
                    .unwrap();
            if out3 != json_val {
                println!("Got:\n{}-----\nExpected:\n{}-----", out3, json_val);
                panic!("Test failed: {:?}", file.file_name());
            }
        }

        Ok(())
    }

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
        , "is": "a", "v":{"another" :"object",}  },
        } // Trailing comment."#;
        let mut buf = String::new();
        println!("{}", input);
        if let Err(err) = format_jsonc(Scanner::new(input), &mut buf) {
            println!("ERROR: {}", err);
        }
        println!("-----\n{}-----", buf);
        let mut buf2 = String::new();
        if let Err(err) = format_jsonc(Scanner::new(buf.as_str()), &mut buf2) {
            println!("ERROR: {}", err);
        }
        //println!("-----\n{}-----", buf2);
        assert!(buf == buf2);
        buf2.clear();
        if let Err(err) = format_json(Scanner::new(input), &mut buf2) {
            println!("ERROR: {}", err);
        }
        println!("-----\n{}-----", buf2);
    }
}
