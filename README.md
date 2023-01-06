# fjson

A Rust library for parsing and formatting JSON with C-style comments and
trailing commas

[![](https://img.shields.io/crates/v/fjson.svg)](https://crates.io/crates/fjson)

## Usage

Given the following input:

```jsonc
// This is a JSON value with comments and trailing commas
{
    /* The project name is fjson */
    "project": "fjson",
    "language": "Rust",
    "license": [
        "MIT",
    ],


    // This project is public.
    "public": true,
}
```

## Format as JSONC

Format to pretty JSONC, intended for human viewing:

```rust
let output = fjson::format_jsonc(input).unwrap();
println!("{}", output);
```

Prints:

```jsonc
// This is a JSON value with comments and trailing commas
{
  /* The project name is fjson */
  "project": "fjson",
  "language": "Rust",
  "license": ["MIT"],

  // This project is public.
  "public": true
}
```


## Format as valid, pretty JSON

Format to pretty JSON, intended for human viewing:

```rust
let output = fjson::format_json(input)?;
println!("{}", output);
```

Prints:

```jsonc
{
  "project": "fjson",
  "language": "Rust",
  "license": ["MIT"],
  "public": true
}
```

## Format as valid, compact JSON

Format to compact JSON, intended for computer consumption:

```rust
let output = fjson::format_json_compact(input)?;
println!("{}", output);
```

Prints:

```json
{"project":"fjson","language":"Rust","license":["MIT"],"public":true}
```

## Deserialize with [Serde](https://serde.rs/)

To parse JSON with C-style comments and trailing commas, but deserialize via
serde, this can be accomplished with the following:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Project {
    project: String,
    language: String,
    license: Vec<String>,
    public: bool,
}

let output = fjson::format_json_compact(input)?;
let project: Project = serde_json::from_str(&output)?;
println!("{:#?}", project);
```
