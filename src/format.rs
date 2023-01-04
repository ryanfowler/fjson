use std::fmt::{Error, Write};

use crate::ast::{
    strip_metadata, ArrayValue, Comment, Metadata, ObjectValue, Root, Value, ValueToken,
};

pub fn write_json<W: Write>(w: &mut W, root: &mut Root) -> Result<(), Error> {
    strip_metadata(root);
    write_jsonc(w, root)
}

pub fn write_jsonc<W: Write>(w: &mut W, root: &Root) -> Result<(), Error> {
    let mut ctx = Context { w, written: 0 };
    for meta in &root.meta_above {
        ctx.write_metadata(meta)?;
        ctx.write_newline()?;
    }
    ctx.write_value(&root.value.token, 0)?;
    ctx.write_comments(&root.value.comments)?;
    for meta in &root.meta_below {
        ctx.write_metadata(meta)?;
        ctx.write_newline()?;
    }
    ctx.write_newline()
}

struct Context<'a, W: Write> {
    w: &'a mut W,
    written: usize,
}

impl<'a, W: Write> Context<'a, W> {
    fn write_value(&mut self, value: &ValueToken, indent: usize) -> Result<(), Error> {
        match value {
            ValueToken::Object(vals) => self.write_json_object(vals, indent),
            ValueToken::Array(vals) => self.write_json_array(vals, indent),
            ValueToken::String(v) => self.write_json_string(v),
            ValueToken::Number(v) => self.write_str(v),
            ValueToken::Bool(v) => self.write_json_bool(*v),
            ValueToken::Null => self.write_str("null"),
        }
    }

    fn write_json_object(&mut self, vals: &[ObjectValue], indent: usize) -> Result<(), Error> {
        let length = vals.len();
        let same_line = LINE_LENGTH > self.written()
            && can_fit_object(vals, LINE_LENGTH - self.written()).is_some();

        self.write_char('{')?;
        for (i, val) in vals.iter().enumerate() {
            if same_line {
                self.write_char(' ')?;
            } else {
                self.write_newline()?;
                self.write_indent(indent + 1)?;
            }
            match val {
                ObjectValue::KeyVal(k, v) => {
                    self.write_json_string(k)?;
                    self.write_str(": ")?;
                    self.write_value(&v.token, indent + 1)?;
                    if i < length - 1 {
                        self.write_char(',')?;
                    }
                    self.write_comments(&v.comments)?;
                }
                ObjectValue::Metadata(meta) => self.write_metadata(meta)?,
            }
        }
        if length > 0 {
            if same_line {
                self.write_char(' ')?;
            } else {
                self.write_newline()?;
                self.write_indent(indent)?;
            }
        }
        self.write_char('}')
    }
    fn write_json_array(&mut self, vals: &[ArrayValue], indent: usize) -> Result<(), Error> {
        let length = vals.len();
        let same_line = LINE_LENGTH > self.written()
            && can_fit_array(vals, LINE_LENGTH - self.written()).is_some();

        self.write_char('[')?;
        for (i, val) in vals.iter().enumerate() {
            if same_line {
                if i > 0 {
                    self.write_char(' ')?;
                }
            } else {
                self.write_newline()?;
                self.write_indent(indent + 1)?;
            }
            match val {
                ArrayValue::ArrayVal(v) => {
                    self.write_value(&v.token, indent + 1)?;
                    if i < length - 1 {
                        self.write_char(',')?;
                    }
                    self.write_comments(&v.comments)?;
                }
                ArrayValue::Metadata(meta) => self.write_metadata(meta)?,
            }
        }
        if length > 0 && !same_line {
            self.write_newline()?;
            self.write_indent(indent)?;
        }
        self.write_char(']')
    }

    fn write_json_bool(&mut self, v: bool) -> Result<(), Error> {
        if v {
            self.write_str("true")
        } else {
            self.write_str("false")
        }
    }

    fn written(&self) -> usize {
        self.written
    }

    fn write_metadata(&mut self, meta: &Metadata) -> Result<(), Error> {
        if let Metadata::Comment(c) = meta {
            self.write_comment(c)?;
        }
        Ok(())
    }

    fn write_comments(&mut self, cs: &[Comment]) -> Result<(), Error> {
        if cs.is_empty() {
            return Ok(());
        }
        self.write_char(' ')?;
        for comment in cs {
            self.write_comment(comment)?;
        }
        Ok(())
    }

    fn write_comment(&mut self, comment: &Comment) -> Result<(), Error> {
        match comment {
            Comment::Block(c) => {
                // Do we need to look for newlines and adjust self.written?
                self.write_str("/*")?;
                self.write_str(c)?;
                self.write_str("*/")
            }
            Comment::Line(c) => {
                self.write_str("//")?;
                self.write_str(c)
            }
        }
    }

    fn write_json_string(&mut self, s: &str) -> Result<(), Error> {
        self.write_char('"')?;
        self.write_str(s)?;
        self.write_char('"')
    }

    fn write_indent(&mut self, n: usize) -> Result<(), Error> {
        for _ in 0..n {
            self.write_str(INDENT)?;
        }
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.w.write_str(s)?;
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

fn can_fit_value(val: &ValueToken, space: usize) -> Option<usize> {
    let remaining = space as i64;
    let remaining = match val {
        ValueToken::Object(v) => return can_fit_object(v, space),
        ValueToken::Array(v) => return can_fit_array(v, space),
        ValueToken::String(v) => remaining - (2 + v.len() as i64),
        ValueToken::Number(v) => remaining - v.len() as i64,
        ValueToken::Bool(v) => {
            if *v {
                remaining - 4
            } else {
                remaining - 5
            }
        }
        ValueToken::Null => remaining - 4,
    };
    if remaining >= 0 {
        Some(remaining as usize)
    } else {
        None
    }
}

fn can_fit_object(vals: &[ObjectValue], space: usize) -> Option<usize> {
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
                match can_fit_value(&v.token, remaining as usize) {
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

fn can_fit_array(vals: &[ArrayValue], space: usize) -> Option<usize> {
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
                match can_fit_value(&v.token, remaining as usize) {
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

pub fn write_json_compact<W: Write>(w: &mut W, root: &Root) -> Result<(), Error> {
    write_json_value_compact(w, &root.value)
}

pub fn write_json_value_compact<W: Write>(w: &mut W, value: &Value) -> Result<(), Error> {
    match &value.token {
        ValueToken::Object(vals) => {
            w.write_char('{')?;
            let mut first = true;
            for val in vals {
                if let ObjectValue::KeyVal(k, v) = val {
                    if first {
                        first = false;
                    } else {
                        w.write_char(',')?;
                    }
                    w.write_char('"')?;
                    w.write_str(k)?;
                    w.write_str("\":")?;
                    write_json_value_compact(w, v)?;
                }
            }
            w.write_char('}')?;
        }
        ValueToken::Array(vals) => {
            w.write_char('[')?;
            let mut first = true;
            for val in vals {
                if let ArrayValue::ArrayVal(v) = val {
                    if first {
                        first = false;
                    } else {
                        w.write_char(',')?;
                    }
                    write_json_value_compact(w, v)?;
                }
            }
            w.write_char(']')?;
        }
        ValueToken::String(v) => {
            w.write_char('"')?;
            w.write_str(v)?;
            w.write_char('"')?;
        }
        ValueToken::Number(v) => w.write_str(v)?,
        ValueToken::Bool(v) => {
            if *v {
                w.write_str("true")?;
            } else {
                w.write_str("false")?;
            }
        }
        ValueToken::Null => w.write_str("null")?,
    }
    Ok(())
}
/*
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
*/
