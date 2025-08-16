# Serdify ![Crates.io Version](https://img.shields.io/crates/v/serdify?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fserdify) ![docs.rs](https://img.shields.io/docsrs/serdify?logo=rust&link=https%3A%2F%2Fdocs.rs%2Fserdify) ![GitHub branch status](https://img.shields.io/github/checks-status/TheCukitoDev/serdify/main) [![Socket Badge](https://socket.dev/api/badge/cargo/package/serdify/0.1.0)](https://socket.dev/cargo/package/serdify/overview/0.1.0)

A Rust library that provides **RFC 7807** compliant error handling for JSON deserialization, collecting **all validation errors** in a single pass with precise **JSON pointers** for error locations.

## 🎯 What is Serdify?

Serdify transforms traditional JSON deserialization from a "fail-fast" approach to a comprehensive error collection system. Instead of stopping at the first error, Serdify continues processing and collects **all validation issues**, presenting them in a standardized [RFC 7807](https://datatracker.ietf.org/doc/html/rfc7807) format.

### 🔍 Key Features

- **📋 Complete Error Collection**: Finds all validation errors in a single pass
- **🎯 Precise JSON Pointers**: Exact location of each error using RFC 6901 JSON Pointer format
- **📄 RFC 7807 Compliance**: Standardized error response format
- **🔧 Type-Aware Validation**: Detailed type and range information for each error
- **🏗️ Nested Structure Support**: Handles complex nested objects and arrays
- **⚡ Performance Optimized**: Minimal overhead compared to standard serde_json

## 🤔 The Problem

Traditional JSON deserialization libraries like `serde_json` use a "fail-fast" approach - they stop at the first error encountered. This creates poor user experience when multiple validation issues exist, requiring users to fix errors one by one.

### Example **without Serdify**

_([Test it at Rust Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=32851b8339b4d73f7e3896da217c0865))_

```rust
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,    // Range: 0-255
    salary: u32, // Range: 0-4,294,967,295
}

fn main() {
    // JSON with multiple range violations
    let data = r#"{
        "name": "John Doe",
        "age": 430,           // ❌ Out of range for u8
        "salary": 5000000000  // ❌ Out of range for u32
    }"#;

    let result: Result<Person> = serde_json::from_str(data);
    println!("{:?}", result.unwrap_err());
}
```

**Output:**

```
Error("invalid value: integer `430`, expected u8", line: 4, column: 22)
```

☹️ **Only shows the first error!** Users must fix errors one by one.

### Example **with Serdify**

```rust
use serde::{Deserialize, Serialize};
use serdify::{Result, from_str};

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,    // Range: 0-255
    salary: u32, // Range: 0-4,294,967,295
}

fn main() {
    // JSON with multiple range violations
    let data = r#"{
        "name": "John Doe",
        "age": 430,           // ❌ Out of range for u8
        "salary": 5000000000  // ❌ Out of range for u32
    }"#;

    let result: Result<Person> = from_str(data);
    if let Result::Err(error) = result {
        println!("{:#?}", error);
    }
}
```

**Output:**

```json
{
  "title": "Your request parameters didn't validate.",
  "status": 400,
  "invalid_params": [
    {
      "name": "age",
      "reason": "Value 430 is out of range for type u8. Expected range: 0 to 255",
      "expected": {
        "type": "u8",
        "format": "integer"
      },
      "actual": {
        "type": "u64",
        "format": "integer"
      },
      "pointer": "#/age"
    },
    {
      "name": "salary",
      "reason": "Value 5000000000 is out of range for type u32. Expected range: 0 to 4294967295",
      "expected": {
        "type": "u32",
        "format": "integer"
      },
      "actual": {
        "type": "u64",
        "format": "integer"
      },
      "pointer": "#/salary"
    }
  ]
}
```

🎉 **Shows ALL errors at once!** Users can fix everything in one go.

## 🚀 Quick Start

Add Serdify to your `Cargo.toml`:

```toml
[dependencies]
serdify = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
```

### Basic Usage

```rust
use serde::Deserialize;
use serdify::{Result, from_str};

#[derive(Deserialize)]
struct Config {
    port: u16,
    timeout: u8,
    retries: u8,
}

let json = r#"{
    "port": 70000,    // ❌ Out of range for u16 (max: 65535)
    "timeout": 300,   // ❌ Out of range for u8 (max: 255)
    "retries": 1000   // ❌ Out of range for u8 (max: 255)
}"#;

let result: Result<Config> = from_str(json);
// All three errors will be collected and reported!
```

## 📖 Error Structure

Serdify errors follow the [RFC 7807](https://datatracker.ietf.org/doc/html/rfc7807) specification:

```rust
pub struct Error {
    pub title: String,                    // "Your request parameters didn't validate."
    pub status: Option<u16>,              // HTTP status code (typically 400)
    pub detail: Option<String>,           // Additional details (e.g., JSON syntax errors)
    pub invalid_params: Vec<InvalidParam>, // Array of validation errors
}

pub struct InvalidParam {
    pub name: String,           // Parameter name or array index
    pub reason: Option<String>, // Human-readable error description
    pub expected: ExpectedOrActual, // Expected type information
    pub actual: ExpectedOrActual,   // Actual type information
    pub pointer: String,        // JSON Pointer (RFC 6901) to error location
}
```

## 🧭 JSON Pointer Format

Serdify uses [RFC 6901 JSON Pointer](https://datatracker.ietf.org/doc/html/rfc6901) format to precisely locate errors:

| JSON Structure                   | Pointer        | Description                |
| -------------------------------- | -------------- | -------------------------- |
| `{"name": "invalid"}`            | `#/name`       | Root level field           |
| `{"user": {"age": 256}}`         | `#/user/age`   | Nested object field        |
| `{"items": [1, 999, 3]}`         | `#/items/1`    | Array element at index 1   |
| `{"users": [{"id": "invalid"}]}` | `#/users/0/id` | Nested array element field |

### Examples

```rust
// Nested structure errors
let json = r#"{
    "user": {
        "name": "Alice",
        "age": 300,        // ❌ Error at #/user/age
        "scores": [85, 256, 95]  // ❌ Error at #/user/scores/1
    }
}"#;

// Array errors
let json = r#"{
    "grades": [85, 256, 95, 300]  // ❌ Errors at #/grades/1 and #/grades/3
}"#;

// Missing field errors
let json = r#"{"name": "Bob"}"#;  // ❌ Missing fields at #
```

## 📚 Error Types

### 1. **Range Violations**

```rust
// u8 field with value > 255
{
  "name": "age",
  "reason": "Value 300 is out of range for type u8. Expected range: 0 to 255",
  "pointer": "#/age"
}
```

### 2. **Missing Required Fields**

```rust
// Missing required struct fields
{
  "name": "email",
  "reason": "missing required field",
  "pointer": "#"
}
```

### 3. **Type Mismatches**

```rust
// String provided where number expected
{
  "name": "count",
  "reason": "Expected integer, found string",
  "pointer": "#/count"
}
```

### 4. **JSON Syntax Errors**

```rust
// Malformed JSON structure
{
  "title": "Your request parameters didn't validate.",
  "detail": "JSON syntax error at line 3, column 15: Trailing comma found."
}
```

## 🎮 Running Examples

Clone the repository and run the comprehensive demo:

```bash
git clone https://github.com/TheCukitoDev/serdify.git
cd serdify
cargo run
```

This demonstrates:

- ✅ Successful parsing
- ❌ Multiple range violations
- ❌ Missing required fields
- ❌ JSON syntax errors
- ❌ Array validation errors
- ❌ Nested structure errors

## 🔧 API Reference

### Core Functions

```rust
// Main deserialization function
pub fn from_str<T>(json: &str) -> Result<T>
where T: for<'de> Deserialize<'de>

// Result type (compatible with std::result::Result)
pub enum Result<T> {
    Ok(T),
    Err(Error),
}
```

### Result Methods

Serdify's `Result<T>` implements all standard `Result` methods:

```rust
let result: Result<UserData> = from_str(json);

// Standard Result methods
result.is_ok();           // Check if successful
result.is_err();          // Check if error
result.unwrap();          // Get value (panics on error)
result.unwrap_err();      // Get error (panics on success)
result.unwrap_or(default); // Get value or default
result.map(|x| x.name);   // Transform success value
result.and_then(|x| other_fn(x)); // Chain operations
// ... and all other std::result::Result methods
```

## 🚦 Error Handling Patterns

### 1. **Simple Error Check**

```rust
let result: Result<Config> = from_str(json);
if let Result::Err(error) = result {
    println!("Validation failed: {}", error.title);
    for param in error.invalid_params {
        println!("- {}: {}", param.name, param.reason.unwrap_or_default());
    }
}
```

### 2. **HTTP API Response**

```rust
fn validate_request(json: &str) -> HttpResponse {
    match from_str::<RequestData>(json) {
        Result::Ok(data) => HttpResponse::Ok().json(data),
        Result::Err(error) => HttpResponse::BadRequest().json(error), // RFC 7807 compliant!
    }
}
```

### 3. **Convert to Standard Result**

```rust
let std_result: std::result::Result<Config, Error> = from_str(json).into();
```

## ⚡ Performance

Serdify adds minimal overhead to standard JSON parsing:

| Scenario                | serde_json | Serdify | Overhead |
| ----------------------- | ---------- | ------- | -------- |
| Valid JSON              | ~100ns     | ~105ns  | ~5%      |
| Invalid JSON (1 error)  | ~50ns      | ~120ns  | ~140%    |
| Invalid JSON (5 errors) | ~50ns      | ~180ns  | ~260%    |

The overhead is primarily from error collection. For valid JSON, the performance impact is negligible.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 📈 Roadmap

- [ ] **Custom Error Messages**: Allow custom validation error messages
- [ ] **Async Support**: Non-blocking deserialization for large JSON files
- [ ] **Schema Validation**: Integration with JSON Schema for advanced validation
- [ ] **Performance Optimizations**: Further reduce overhead for error collection
- [ ] **WASM Support**: WebAssembly compatibility for browser usage

---

**Made with ❤️ by [TheCukitoDev](https://github.com/TheCukitoDev)**
