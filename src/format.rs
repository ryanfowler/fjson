//! Format `Root` values to JSONC or pretty/compact JSON.

use std::fmt::{Error, Write};

use crate::{
    ast::{ArrayValue, Comment, Metadata, ObjectValue, Root, Value, ValueToken},
    scanner::{ScanResult, Token},
    validate::ValidateIter,
};

/// Options represents the customizations that can be made when formatting.
#[derive(Debug, Copy, Clone)]
pub struct Options<'a> {
    indent: &'a str,
    line_length: usize,
    max_object_pairs_per_line: usize,
    max_array_values_per_line: usize,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            indent: "  ",
            line_length: 80,
            max_object_pairs_per_line: 1,
            max_array_values_per_line: 4,
        }
    }
}

impl<'a> Options<'a> {
    /// Sets the indent to the provided string. The default is two spaces.
    pub fn with_indent(self, s: &'a str) -> Self {
        Self { indent: s, ..self }
    }

    /// Sets the line length that objects and arrays will wrap on. The default
    /// is 80 characters.
    pub fn with_line_length(self, n: usize) -> Self {
        Self {
            line_length: n,
            ..self
        }
    }

    /// Sets the maximum number of object key/value pairs that can appear on the
    /// same line. The default is 1.
    pub fn with_max_object_pairs_per_line(self, n: usize) -> Self {
        Self {
            max_object_pairs_per_line: n,
            ..self
        }
    }

    /// Sets the maximum number of array values that can appear on the same
    /// line. The default is 4.
    pub fn with_max_array_values_per_line(self, n: usize) -> Self {
        Self {
            max_array_values_per_line: n,
            ..self
        }
    }
}

/// Serializes/formats the provided JSON [Root] value to the writer as "jsonc".
///
/// The output will be formatted according to a number of rules and is intended
/// for human viewing.
pub fn write_jsonc<W: Write>(w: &mut W, root: &Root) -> Result<(), Error> {
    write_jsonc_opts(w, root, &Options::default())
}

/// Serializes/formats the provided JSON [Root] value to the writer as "jsonc"
/// using the formatting options.
///
/// The output written to `w` is intended for human viewing.
pub fn write_jsonc_opts<W: Write>(w: &mut W, root: &Root, opts: &Options) -> Result<(), Error> {
    let mut ctx = Context {
        w,
        current_line_chars: 0,
        opts: *opts,
    };
    for meta in &root.meta_above {
        ctx.write_metadata(meta)?;
        ctx.write_newline()?;
    }
    ctx.write_value(&root.value.token, 0, false)?;
    ctx.write_comments(&root.value.comments)?;
    for meta in &root.meta_below {
        ctx.write_metadata(meta)?;
        ctx.write_newline()?;
    }
    ctx.write_newline()
}

struct Context<'a, W: Write> {
    w: &'a mut W,
    current_line_chars: usize,
    opts: Options<'a>,
}

impl<'a, W: Write> Context<'a, W> {
    fn write_value(
        &mut self,
        value: &ValueToken,
        indent: usize,
        allow_sameline: bool,
    ) -> Result<(), Error> {
        match value {
            ValueToken::Object(vals) => self.write_json_object(vals, indent, allow_sameline),
            ValueToken::Array(vals) => self.write_json_array(vals, indent, allow_sameline),
            ValueToken::String(v) => self.write_json_string(v),
            ValueToken::Number(v) => self.write_str(v),
            ValueToken::Bool(v) => self.write_json_bool(*v),
            ValueToken::Null => self.write_str("null"),
        }
    }

    fn write_json_object(
        &mut self,
        vals: &[ObjectValue],
        indent: usize,
        allow_sameline: bool,
    ) -> Result<(), Error> {
        let length = vals.len();
        let same_line = allow_sameline
            && self.opts.line_length > self.current_line_chars()
            && self
                .can_fit_object(vals, self.opts.line_length - self.current_line_chars())
                .is_some();

        self.write_char('{')?;
        for (i, val) in vals.iter().enumerate() {
            if same_line {
                self.write_char(' ')?;
            } else {
                self.write_newline()?;
                match val {
                    ObjectValue::Metadata(Metadata::Newline) => {}
                    _ => self.write_indent(indent + 1)?,
                }
            }
            match val {
                ObjectValue::KeyVal(k, v) => {
                    self.write_json_string(k)?;
                    self.write_str(": ")?;
                    self.write_value(&v.token, indent + 1, true)?;
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

    fn write_json_array(
        &mut self,
        vals: &[ArrayValue],
        indent: usize,
        allow_sameline: bool,
    ) -> Result<(), Error> {
        let length = vals.len();
        let same_line = allow_sameline
            && self.opts.line_length > self.current_line_chars()
            && self
                .can_fit_array(vals, self.opts.line_length - self.current_line_chars())
                .is_some();

        self.write_char('[')?;
        for (i, val) in vals.iter().enumerate() {
            if same_line {
                if i > 0 {
                    self.write_char(' ')?;
                }
            } else {
                self.write_newline()?;
                match val {
                    ArrayValue::Metadata(Metadata::Newline) => {}
                    _ => self.write_indent(indent + 1)?,
                }
            }
            match val {
                ArrayValue::ArrayVal(v) => {
                    self.write_value(&v.token, indent + 1, true)?;
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

    fn current_line_chars(&self) -> usize {
        self.current_line_chars
    }

    fn write_metadata(&mut self, meta: &Metadata) -> Result<(), Error> {
        if let Metadata::Comment(c) = meta {
            self.write_comment(c)?;
        }
        Ok(())
    }

    fn write_comments(&mut self, cs: &[Comment]) -> Result<(), Error> {
        for comment in cs {
            self.write_char(' ')?;
            self.write_comment(comment)?;
        }
        Ok(())
    }

    fn write_comment(&mut self, comment: &Comment) -> Result<(), Error> {
        match comment {
            Comment::Block(c) => {
                self.write_str("/*")?;
                self.write_str(c)?;
                if let Some(i) = c.rfind('\n') {
                    // If the block comment contains newlines, adjust the
                    // internal value of chars written for the current line.
                    self.current_line_chars = c[(i + 1)..].chars().count();
                }
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
            self.write_str(self.opts.indent)?;
        }
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.w.write_str(s)?;
        self.current_line_chars += s.chars().count();
        Ok(())
    }

    fn write_newline(&mut self) -> Result<(), Error> {
        self.write_char('\n')?;
        self.current_line_chars = 0;
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), Error> {
        self.w.write_char(c)?;
        self.current_line_chars += 1;
        Ok(())
    }

    fn can_fit_value(&self, val: &ValueToken, space: usize) -> Option<usize> {
        let remaining = space as i64;
        let remaining = match val {
            ValueToken::Object(v) => return self.can_fit_object(v, space),
            ValueToken::Array(v) => return self.can_fit_array(v, space),
            ValueToken::String(v) => remaining - (2 + v.chars().count() as i64),
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

    fn can_fit_object(&self, vals: &[ObjectValue], space: usize) -> Option<usize> {
        let num_vals = vals.len();
        if num_vals > self.opts.max_object_pairs_per_line {
            return None;
        }

        let mut remaining = (space as i64) - 2; // For object start/close.
        if !vals.is_empty() {
            // Object padding + (key quotes + colon + padding) * values + (comma + padding) * (values - 1).
            remaining -= 2 + 4 * num_vals as i64 + 2 * (num_vals as i64 - 1);
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
                    remaining -= k.chars().count() as i64;
                    if remaining < 0 {
                        return None;
                    }
                    match v.token {
                        ValueToken::Array(_) => return None,
                        ValueToken::Object(_) => return None,
                        _ => {}
                    }
                    match self.can_fit_value(&v.token, remaining as usize) {
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

    fn can_fit_array(&self, vals: &[ArrayValue], space: usize) -> Option<usize> {
        let num_vals = vals.len();
        if num_vals > self.opts.max_array_values_per_line {
            return None;
        }

        let mut remaining = (space as i64) - 2; // For array start/close.
        if !vals.is_empty() {
            // (comma + padding) * (values - 1).
            remaining -= 2 * (num_vals as i64 - 1);
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
                    match v.token {
                        ValueToken::Array(_) => return None,
                        ValueToken::Object(_) => return None,
                        _ => {}
                    }
                    match self.can_fit_value(&v.token, remaining as usize) {
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
}

/// Serializes/formats the provided `Iterator` of [ScanResult]s to the writer.
///
/// This function will ensure that the provided input is validate JSON(C),
/// returning any error encountered.
///
/// Note: It's more efficient to use this function to serialize compact JSON
/// from an input than parsing a [Root] struct and using the
/// [write_json_compact] function.
pub fn write_json_compact_iter<'a, W, I>(w: &mut W, iter: I) -> Result<(), crate::Error>
where
    W: Write,
    I: Iterator<Item = ScanResult<'a>>,
{
    for result in iter.validate() {
        let event = match result {
            Ok(event) => event,
            Err(err) => return Err(err),
        };
        match event.token {
            Token::ObjectStart => w.write_char('{')?,
            Token::ObjectEnd => w.write_char('}')?,
            Token::ArrayStart => w.write_char('[')?,
            Token::ArrayEnd => w.write_char(']')?,
            Token::Comma => w.write_char(',')?,
            Token::Colon => w.write_char(':')?,
            Token::Null => w.write_str("null")?,
            Token::String(v) => {
                w.write_char('"')?;
                w.write_str(v)?;
                w.write_char('"')?;
            }
            Token::Number(v) => w.write_str(v)?,
            Token::Bool(v) => w.write_str(if v { "true" } else { "false" })?,
            _ => {}
        }
    }
    Ok(())
}

/// Serializes/formats the provided JSON [Root] value to the writer as valid
/// JSON.
///
/// The output will be formatted as valid, compact JSON; intended for
/// consumption by computers.
///
/// Note: It's more efficient to use the [write_json_compact_iter] function to
/// serialize compact JSON from an input than parsing a [Root] struct and using
/// this function.
pub fn write_json_compact<W: Write>(w: &mut W, root: &Root) -> Result<(), Error> {
    write_json_value_compact(w, &root.value)
}

fn write_json_value_compact<W: Write>(w: &mut W, value: &Value) -> Result<(), Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::parse, scanner::Scanner};

    #[test]
    fn test_format() {
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

        let expected_jsonc = r#"// This is a comment.
// Second line.

// Break, than third.

{
  // Object start.

  "key1": "val1", // Same line comment.
  "k": "v",
  // Next line comment.
  "arr_key": [
    // Array start.

    "val1",
    100, // Before comma

    // True.
    true
  ],

  // And another.
  "key2": {
    // And another one.
    "nested": 100,
    "value": true,
    "third": "this",

    // Weird comment before comma.
    "is": "a",
    "v": { "another": "object" }
  }
} // Trailing comment.
"#;
        let root = parse(input).unwrap();

        let mut jsonc = String::new();
        write_jsonc(&mut jsonc, &root).unwrap();
        assert_eq!(&jsonc, expected_jsonc);

        // Parse and reformat the jsonc output. The reformatted output should
        // match the original output.
        let root2 = parse(&jsonc).unwrap();
        let mut jsonc2 = String::new();
        write_jsonc(&mut jsonc2, &root2).unwrap();
        assert_eq!(&jsonc2, &jsonc);

        let expected_json_compact = r#"{"key1":"val1","k":"v","arr_key":["val1",100,true],"key2":{"nested":100,"value":true,"third":"this","is":"a","v":{"another":"object"}}}"#;
        let mut json_compact = String::new();
        write_json_compact(&mut json_compact, &root).unwrap();
        assert_eq!(&json_compact, expected_json_compact);

        // Parse and reformat the json compact output. The reformatted output
        // should match the original output.
        let root2 = parse(&json_compact).unwrap();
        let mut json_compact2 = String::new();
        write_json_compact(&mut json_compact2, &root2).unwrap();
        assert_eq!(&json_compact2, &json_compact);

        let mut json_compact_iter = String::new();
        write_json_compact_iter(&mut json_compact_iter, Scanner::new(input)).unwrap();
        assert_eq!(&json_compact_iter, expected_json_compact);

        let mut json_compact_iter2 = String::new();
        write_json_compact_iter(&mut json_compact_iter2, Scanner::new(&json_compact_iter)).unwrap();
        assert_eq!(&json_compact_iter2, &json_compact_iter);
    }
}
