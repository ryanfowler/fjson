//! # fjson
//!
//! A library for parsing and formatting JSON with C-style comments and trailing
//! commas.
//!
//! Given the following input:
//!
//! ```jsonc
//! // This is a JSON value with comments and trailing commas
//! {
//!     /* The project name is fjson */
//!     "project": "fjson",
//!     "language": "Rust",
//!     "license": [
//!         "MIT",
//!     ],
//!
//!
//!     // This project is public.
//!     "public": true,
//! }
//! ```
//!
//! This value can be formatted to JSONC using the function:
//!
//! ```
//! use fjson::Error;
//!
//! const INPUT: &str = r#"
//! // This is a JSON value with comments and trailing commas
//! {
//!     /* The project name is fjson */
//!     "project": "fjson",
//!     "language": "Rust",
//!     "license": [
//!         "MIT",
//!     ],
//!
//!
//!     // This project is public.
//!     "public": true,
//! }"#;
//!
//! fn main() -> Result<(), Error> {
//!     let output = fjson::format_jsonc(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! This would print:
//!
//! ```jsonc
//! // This is a JSON value with comments and trailing commas
//! {
//!   /* The project name is fjson */
//!   "project": "fjson",
//!   "language": "Rust",
//!   "license": ["MIT"],
//!
//!   // This project is public.
//!   "public": true
//! }
//! ```
//!
//! The value can also be formatted as valid JSON:
//!
//! ```
//! use fjson::Error;
//!
//! const INPUT: &str = r#"
//! // This is a JSON value with comments and trailing commas
//! {
//!     /* The project name is fjson */
//!     "project": "fjson",
//!     "language": "Rust",
//!     "license": [
//!         "MIT",
//!     ],
//!
//!
//!     // This project is public.
//!     "public": true,
//! }"#;
//!
//! fn main() -> Result<(), Error> {
//!     let output = fjson::format_json(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! Which would print:
//!
//! ```jsonc
//! {
//!   "project": "fjson",
//!   "language": "Rust",
//!   "license": ["MIT"],
//!   "public": true
//! }
//! ```
//!
//! Or we can format the input into compact, valid JSON:
//!
//! ```
//! use fjson::Error;
//!
//! const INPUT: &str = r#"
//! // This is a JSON value with comments and trailing commas
//! {
//!     /* The project name is fjson */
//!     "project": "fjson",
//!     "language": "Rust",
//!     "license": [
//!         "MIT",
//!     ],
//!
//!
//!     // This project is public.
//!     "public": true,
//! }"#;
//!
//! fn main() -> Result<(), Error> {
//!     let output = fjson::format_json_compact(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! Printing:
//!
//! ```json
//! {"project":"fjson","language":"Rust","license":["MIT"],"public":true}
//! ```
//!
//! ## Deserialize with [Serde](https://serde.rs/)
//!
//! To parse JSON with C-style comments and trailing commas, but deserialize via
//! serde, the following can be done:
//!
//! ```
//! use serde::Deserialize;
//! use serde_json::from_str;
//!
//! #[derive(Debug, Deserialize)]
//! struct Project {
//!     project: String,
//!     language: String,
//!     license: Vec<String>,
//!     public: bool,
//! }
//!
//! const INPUT: &str = r#"
//! // This is a JSON value with comments and trailing commas
//! {
//!     /* The project name is fjson */
//!     "project": "fjson",
//!     "language": "Rust",
//!     "license": [
//!         "MIT",
//!     ],
//!
//!
//!     // This project is public.
//!     "public": true,
//! }"#;
//!
//! fn main() {
//!     let output = fjson::format_json_compact(INPUT).unwrap();
//!     let project: Project = from_str(&output).unwrap();
//!     println!("{:#?}", project);
//! }
//! ```

#![forbid(unsafe_code)]

pub mod ast;
pub mod error;
pub mod format;
pub mod scanner;

use std::fmt::Write;

pub use error::Error;
use scanner::Scanner;

/// Prases and formats JSON with C-style comments and trailing commas according
/// to internal rules and is intended for human viewing.
pub fn format_jsonc(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len() + 128);
    format_jsonc_writer(&mut out, input)?;
    Ok(out)
}

/// Parses and formats JSON to the provided writer with C-style comments and
/// trailing commas according to internal rules and is intended for human viewing.
pub fn format_jsonc_writer<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    let root = ast::parse(input)?;
    format::write_jsonc(w, &root)?;
    Ok(())
}

/// Parses JSON with C-style comments and trailing commas, and formats into
/// valid "pretty" JSON intended for human viewing.
pub fn format_json(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len() + 128);
    format_json_writer(&mut out, input)?;
    Ok(out)
}

/// Parses JSON with C-style comments and trailing commas, and formats to the
/// provided writer into valid "pretty" JSON intended for human viewing.
pub fn format_json_writer<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    let root = ast::parse_iter(Scanner::new(input).without_metadata())?;
    format::write_jsonc(w, &root)?;
    Ok(())
}

/// Parses JSON with C-style comments and trailing commas, and serializes into
/// valid compact JSON intended for comptuer consumption.
pub fn format_json_compact(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len());
    format_json_writer_compact(&mut out, input)?;
    Ok(out)
}

/// Parses JSON with C-style comments and trailing commas, and serializes to the
/// provided writer into valid compact JSON intended for comptuer consumption.
pub fn format_json_writer_compact<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    let root = ast::parse_iter(Scanner::new(input).without_metadata())?;
    format::write_json_compact(w, &root)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = r#"
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
    , "is": "a", "v":{"another" :"object", }  },
    } // Trailing comment."#;

    #[test]
    fn test_format_jsonc() {
        let expected = r#"// This is a comment.
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
        let out = format_jsonc(INPUT).unwrap();
        assert_eq!(&out, expected);
    }

    #[test]
    fn test_format_json() {
        let expected = r#"{
  "key1": "val1",
  "k": "v",
  "arr_key": ["val1", 100, true],
  "key2": {
    "nested": 100,
    "value": true,
    "third": "this",
    "is": "a",
    "v": { "another": "object" }
  }
}
"#;
        let out = format_json(INPUT).unwrap();
        assert_eq!(&out, expected);
    }

    #[test]
    fn test_format_json_compact() {
        let expected = r#"{"key1":"val1","k":"v","arr_key":["val1",100,true],"key2":{"nested":100,"value":true,"third":"this","is":"a","v":{"another":"object"}}}"#;
        let out = format_json_compact(INPUT).unwrap();
        assert_eq!(&out, expected);
    }
}
