# fjson

A Rust library for parsing and formatting JSON with C-style comments and
trailing commas

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

### Format as JSONC

```rust
let output = fjson::format_jsonc(input).unwrap();
println!("{}", output);
```

Printing:

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

### Format as valid, pretty JSON

```rust
let output = fjson::format_json(input)?;
println!("{}", output);
```

Printing:

```jsonc
{
  "project": "fjson",
  "language": "Rust",
  "license": ["MIT"],
  "public": true
}
```

### Format as valid, compact JSON

```rust
let output = fjson::format_json_compact(input)?;
println!("{}", output);
```

Printing:

```json
{"project":"fjson","language":"Rust","license":["MIT"],"public":true}
```

## Deserialize with [Serde](https://serde.rs/)

To parse JSON with C-style comments and trailing commas, but deserialize via
serde, the following can be done:

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
