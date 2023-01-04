pub mod ast;
pub mod error;
pub mod format;
pub mod scanner;

pub use error::Error;

pub fn format_jsonc(input: &str) -> Result<String, Error> {
    let root = ast::parse(input)?;
    let mut out = String::with_capacity(input.len() + 128);
    format::write_jsonc(&mut out, &root)?;
    Ok(out)
}

pub fn format_json(input: &str) -> Result<String, Error> {
    let mut root = ast::parse(input)?;
    let mut out = String::with_capacity(input.len() + 128);
    format::write_json(&mut out, &mut root)?;
    Ok(out)
}

pub fn format_json_compact(input: &str) -> Result<String, Error> {
    let root = ast::parse(input)?;
    let mut out = String::with_capacity(input.len());
    format::write_json_compact(&mut out, &root)?;
    Ok(out)
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
        , "is": "a", "v":{"another" :"object", "2":2,}  },
        } // Trailing comment."#;
        println!("{}", input);
        let jsonc = format_jsonc(input).unwrap();
        println!("-----jsonc-----\n{}-----", jsonc);
        let json = format_json(input).unwrap();
        println!("-----json-----\n{}-----", json);
        let json_compact = format_json_compact(input).unwrap();
        println!("-----json_compact-----\n{}-----", json_compact);
    }
}
