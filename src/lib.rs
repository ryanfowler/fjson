//! # fjson
//!
//! A library for parsing and formatting JSON with C-style comments and trailing
//! commas.
//!
//! ## Format as JSONC
//!
//! Format to pretty JSONC, intended for human viewing:
//!
//! ```
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
//! fn main() -> Result<(), fjson::Error> {
//!     let output = fjson::to_jsonc(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//!
//! // Outputs:
//! //
//! // // This is a JSON value with comments and trailing commas
//! // {
//! //   /* The project name is fjson */
//! //   "project": "fjson",
//! //   "language": "Rust",
//! //   "license": ["MIT"],
//! //
//! //   // This project is public.
//! //   "public": true
//! // }
//! ```
//!
//! ## Format as JSON
//!
//! Format to pretty JSON, intended for human viewing:
//!
//! ```
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
//! fn main() -> Result<(), fjson::Error> {
//!     let output = fjson::to_json(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//!
//! // Outputs:
//! //
//! // {
//! //   "project": "fjson",
//! //   "language": "Rust",
//! //   "license": ["MIT"],
//! //   "public": true
//! // }
//! ```
//!
//! ## Format as valid, compact JSON
//!
//! Format to compact JSON, intended for computer consumption:
//!
//! ```
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
//! fn main() -> Result<(), fjson::Error> {
//!     let output = fjson::to_json_compact(INPUT)?;
//!     println!("{}", output);
//!     Ok(())
//! }
//!
//! // Outputs:
//! //
//! // {"project":"fjson","language":"Rust","license":["MIT"],"public":true}
//! ```
//!
//! ## Deserialize with [Serde](https://serde.rs/)
//!
//! To parse JSON with C-style comments and trailing commas, but deserialize via
//! serde, the following can be done:
//!
//! ```
//! use serde::Deserialize;
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
//!     let output = fjson::to_json_compact(INPUT).unwrap();
//!     let project: Project = serde_json::from_str(&output).unwrap();
//!     println!("{:#?}", project);
//! }
//! ```

#![forbid(unsafe_code)]

pub mod ast;
pub mod error;
pub mod format;
pub mod scanner;
pub mod validate;

use std::fmt::Write;

pub use error::Error;
use scanner::Scanner;

/// Parses and formats JSON with C-style comments and trailing commas.
///
/// The output is formatted according to the default options and is intended for
/// consumption by humans.
pub fn to_jsonc(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len() + 128);
    to_jsonc_writer(&mut out, input)?;
    Ok(out)
}

/// Parses and formats JSON with C-style comments and trailing commas to the
/// provided writer.
///
/// The output is formatted according to the default options and is intended for
/// consumption by humans.
pub fn to_jsonc_writer<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    let root = ast::parse(input)?;
    format::write_jsonc(w, &root)?;
    Ok(())
}

/// Parses JSONC and formats the output into "pretty" printed JSON.
///
/// All comments and whitespace are stripped from the input and is formatted
/// according to the default options.
pub fn to_json(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len() + 128);
    to_json_writer(&mut out, input)?;
    Ok(out)
}

/// Parses JSONC and formats the output into "pretty" printed JSON to the
/// provided writer.
///
/// All comments and whitespace are stripped from the input and is formatted
/// according to the default options.
pub fn to_json_writer<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    let root = ast::parse_iter(Scanner::new(input).without_metadata())?;
    format::write_jsonc(w, &root)?;
    Ok(())
}

/// Parses JSONC and formats the output into valid, compact JSON.
///
/// All comments and whitespace are stripped from the input and is formatted to
/// be compact JSON, not intended for consumption by humans.
pub fn to_json_compact(input: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(input.len());
    to_json_writer_compact(&mut out, input)?;
    Ok(out)
}

/// Parses JSONC and formats the output into valid, compact JSON to the provided
/// writer.
///
/// All comments and whitespace are stripped from the input and is formatted to
/// be compact JSON, not intended for consumption by humans.
pub fn to_json_writer_compact<W: Write>(w: &mut W, input: &str) -> Result<(), Error> {
    format::write_json_compact_iter(w, Scanner::new(input).without_metadata())?;
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
    fn test_to_jsonc() {
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
        let out = to_jsonc(INPUT).unwrap();
        assert_eq!(&out, expected);
        let out2 = to_jsonc(&out).unwrap();
        assert_eq!(&out2, &out);
    }

    #[test]
    fn test_to_json() {
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
        let out = to_json(INPUT).unwrap();
        assert_eq!(&out, expected);
        let out2 = to_json(&out).unwrap();
        assert_eq!(&out2, &out);
        let _: serde_json::Value = serde_json::from_str(&out).expect("unable to parse json output");
    }

    #[test]
    fn test_to_json_compact() {
        let expected = r#"{"key1":"val1","k":"v","arr_key":["val1",100,true],"key2":{"nested":100,"value":true,"third":"this","is":"a","v":{"another":"object"}}}"#;
        let out = to_json_compact(INPUT).unwrap();
        assert_eq!(&out, expected);
        let out2 = to_json_compact(&out).unwrap();
        assert_eq!(&out2, &out);
        let _: serde_json::Value = serde_json::from_str(&out).expect("unable to parse json output");
    }
}
