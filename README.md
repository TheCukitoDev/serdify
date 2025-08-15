# Serdify ![Crates.io Version](https://img.shields.io/crates/v/serdify?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fserdify) ![docs.rs](https://img.shields.io/docsrs/serdify?logo=rust&link=https%3A%2F%2Fdocs.rs%2Fserdify) ![GitHub branch status](https://img.shields.io/github/checks-status/TheCukitoDev/serdify/main) [![Socket Badge](https://socket.dev/api/badge/cargo/package/serdify/0.1.0)](https://socket.dev/cargo/package/serdify/overview/0.1.0)

## What is Serdify?

Serdify is a Rust library designed to simplify error handling when working with JSON objects. It provides a set of utilities to help developers manage and transform errors that may occur during the serialization and deserialization process, following the [RFC 7807 specification](https://datatracker.ietf.org/doc/html/rfc7807) for standardized error responses with detailed invalid parameter information.

## The problem

When working with JSON objects, it is common to encounter errors during serialization or deserialization. If you're using a Deserializer on a REST API endpoint returning a single and simple string could lead into a issue for those who consume your API. When using serdify, you will be able to transform traditional `serde_json` errors into pretty [RFC 7807](https://datatracker.ietf.org/doc/html/rfc7807) errors.

Example _**withouth serdify**_ (can be tested [at the rust playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=32851b8339b4d73f7e3896da217c0865)):

```rust
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8, // <-- Field is u8 (0–255). Any value outside this range will cause a deserialization error.
    phones: Vec<String>,
}

fn typed_example() -> Result<()> {
    // JSON input. The "age" field contains a value that does NOT fit into a u8.
    let data = r#"
        {
            "name": "John Doe",
            "age": 430, // <-- ERROR: 430 is out of range for u8 (0 - 255).
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // serde_json tries to deserialize the data into the Person struct.
    // It fails because of the overflow so it returns an error

    let p: Person = serde_json::from_str(data)?;

    println!("Please call {} at the number {}", p.name, p.phones[0]);

    Ok(())
}

fn main () {
    // Using unwarp_err() to prevent rust to panic!() because a error is going to occur.
    let result = typed_example().unwrap_err();

    println!("{result:?}")
}
```

Once we execute the file, we will get this `Standard Output`:

```text
Error("invalid value: integer `430`, expected u8", line: 4, column: 22)
```

Example **with serdify**:

```rust
use serde::{Deserialize, Serialize};
use serdify::Result;

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8, // <-- Field is u8 (0–255). Any value outside this range will cause a deserialization error.
    phones: Vec<String>,
}

fn typed_example() -> Result<()> {
    // JSON input. The "age" field contains a value that does NOT fit into a u8.
    let data = r#"
        {
            "name": "John Doe",
            "age": 430, // <-- ERROR: 430 is out of range for u8 (0 - 255).
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // serdify tries to deserialize the data into the Person struct.
    // It fails because of the overflow so it returns an error

    let p: Person = serdify::from_str(data)?;

    println!("Please call {} at the number {}", p.name, p.phones[0]);

    Ok(())
}

fn main () {
    // Using unwarp_err() to prevent rust to panic!() because a error is going to occur.
    let result = typed_example().unwrap_err();

    println!("{result:?}")
}
```

Once we execute the file, we will get this `Standard Output`:

```json

```
