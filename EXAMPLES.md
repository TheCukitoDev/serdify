# Serdify Examples

This document provides comprehensive examples of using Serdify for RFC 7807 compliant JSON error handling.

## Table of Contents

1. [Basic Examples](#basic-examples)
2. [Error Types](#error-types)
3. [JSON Pointer Examples](#json-pointer-examples)
4. [Complex Scenarios](#complex-scenarios)
5. [API Integration](#api-integration)

## Basic Examples

### Simple Range Violation

```rust
use serde::Deserialize;
use serdify::{Result, from_str};

#[derive(Deserialize)]
struct User {
    name: String,
    age: u8,  // Range: 0-255
}

let json = r#"{"name": "Alice", "age": 300}"#;
let result: Result<User> = from_str(json);

if let Result::Err(error) = result {
    println!("{:#?}", error);
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
      "reason": "Value 300 is out of range for type u8. Expected range: 0 to 255",
      "expected": {
        "type": "u8",
        "format": "integer"
      },
      "actual": {
        "type": "u64",
        "format": "integer"
      },
      "pointer": "#/age"
    }
  ]
}
```

### Multiple Errors Collection

```rust
#[derive(Deserialize)]
struct Config {
    port: u16,      // Range: 0-65535
    timeout: u8,    // Range: 0-255
    retries: u8,    // Range: 0-255
}

let json = r#"{
    "port": 70000,
    "timeout": 300,
    "retries": 1000
}"#;

let result: Result<Config> = from_str(json);
// All three errors will be collected!
```

## Error Types

### 1. Range Violations

Different numeric types have different valid ranges:

```rust
#[derive(Deserialize)]
struct Ranges {
    small: u8,    // 0 to 255
    medium: u16,  // 0 to 65,535
    large: u32,   // 0 to 4,294,967,295
    signed: i8,   // -128 to 127
}

let json = r#"{
    "small": 256,
    "medium": 70000,
    "large": 5000000000,
    "signed": 200
}"#;
```

### 2. Missing Required Fields

```rust
#[derive(Deserialize)]
struct Required {
    name: String,     // Required
    email: String,    // Required
    age: u8,         // Required
}

let json = r#"{"name": "Bob"}"#;  // Missing email and age
```

### 3. Type Mismatches

```rust
#[derive(Deserialize)]
struct TypedData {
    count: u32,
    active: bool,
    score: f64,
}

let json = r#"{
    "count": "not a number",
    "active": "yes",
    "score": "high"
}"#;
```

### 4. JSON Syntax Errors

```rust
// Various JSON syntax issues
let invalid_jsons = [
    r#"{"name": "Bob",}"#,           // Trailing comma
    r#"{"name": "Bob" "age": 25}"#,  // Missing comma
    r#"{"name": "Bob", "age":}"#,    // Missing value
    r#"{"name": "Bob"#,              // Unclosed object
];
```

## JSON Pointer Examples

### Simple Object Fields

```rust
#[derive(Deserialize)]
struct User {
    name: String,
    age: u8,
}

let json = r#"{"name": "Alice", "age": 300}"#;
// Error pointer: "#/age"
```

### Nested Objects

```rust
#[derive(Deserialize)]
struct Profile {
    user: User,
    settings: Settings,
}

#[derive(Deserialize)]
struct User {
    name: String,
    age: u8,
}

#[derive(Deserialize)]
struct Settings {
    theme: String,
    notifications: bool,
}

let json = r#"{
    "user": {
        "name": "Bob",
        "age": 300
    },
    "settings": {
        "theme": "dark",
        "notifications": "yes"
    }
}"#;

// Error pointers:
// "#/user/age" - for the age field
// "#/settings/notifications" - for the notifications field
```

### Arrays

```rust
#[derive(Deserialize)]
struct Scores {
    values: Vec<u8>,
}

let json = r#"{
    "values": [85, 256, 95, 300, 75]
}"#;

// Error pointers:
// "#/values/1" - for index 1 (value 256)
// "#/values/3" - for index 3 (value 300)
```

### Complex Nested Arrays

```rust
#[derive(Deserialize)]
struct School {
    classes: Vec<Class>,
}

#[derive(Deserialize)]
struct Class {
    name: String,
    students: Vec<Student>,
}

#[derive(Deserialize)]
struct Student {
    name: String,
    grade: u8,
}

let json = r#"{
    "classes": [
        {
            "name": "Math",
            "students": [
                {"name": "Alice", "grade": 95},
                {"name": "Bob", "grade": 256}
            ]
        }
    ]
}"#;

// Error pointer: "#/classes/0/students/1/grade"
```

## Complex Scenarios

### Mixed Error Types

```rust
#[derive(Deserialize)]
struct ComplexData {
    config: Config,
    users: Vec<User>,
    metadata: Option<Metadata>,
}

#[derive(Deserialize)]
struct Config {
    port: u16,
    timeout: u8,
}

#[derive(Deserialize)]
struct User {
    id: u32,
    name: String,
    age: u8,
}

#[derive(Deserialize)]
struct Metadata {
    version: String,
    created_at: String,
}

let json = r#"{
    "config": {
        "port": 70000,
        "timeout": 300
    },
    "users": [
        {"id": 1, "name": "Alice", "age": 25},
        {"id": 5000000000, "name": "Bob", "age": 256},
        {"name": "Charlie", "age": 30}
    ],
    "metadata": {
        "version": "1.0"
    }
}"#;

// This will generate errors for:
// - #/config/port (range violation)
// - #/config/timeout (range violation)
// - #/users/1/id (range violation)
// - #/users/1/age (range violation)
// - #/users/2/id (missing field)
// - #/metadata/created_at (missing field)
```

## API Integration

### HTTP API Error Response

```rust
use serdify::{Result, from_str, Error};
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
    age: u8,
}

fn create_user_handler(json_body: &str) -> HttpResponse {
    match from_str::<CreateUserRequest>(json_body) {
        Result::Ok(user_data) => {
            // Process valid user data
            HttpResponse::Created().json(create_user(user_data))
        }
        Result::Err(error) => {
            // Return RFC 7807 compliant error
            HttpResponse::BadRequest()
                .content_type("application/problem+json")
                .json(error)
        }
    }
}
```

### Error Logging

```rust
fn log_validation_errors(error: &Error) {
    log::warn!("Validation failed: {}", error.title);

    for param in &error.invalid_params {
        log::debug!(
            "Invalid parameter '{}' at {}: {}",
            param.name,
            param.pointer,
            param.reason.as_ref().unwrap_or(&"No reason".to_string())
        );
    }
}
```

### Converting to Standard Result

```rust
use std::result::Result as StdResult;

fn process_data(json: &str) -> StdResult<ProcessedData, ValidationError> {
    let serdify_result: Result<RawData> = from_str(json);

    match serdify_result {
        Result::Ok(data) => Ok(process(data)),
        Result::Err(error) => Err(ValidationError::from(error)),
    }
}

impl From<Error> for ValidationError {
    fn from(error: Error) -> Self {
        ValidationError {
            message: error.title,
            details: error.invalid_params.into_iter()
                .map(|p| format!("{}: {}", p.pointer, p.reason.unwrap_or_default()))
                .collect(),
        }
    }
}
```

### Custom Error Handling

```rust
fn handle_errors(result: Result<UserData>) -> AppResult<UserData> {
    match result {
        Result::Ok(data) => Ok(data),
        Result::Err(error) => {
            // Log for debugging
            log_validation_errors(&error);

            // Convert to application-specific error
            let field_errors: Vec<FieldError> = error.invalid_params
                .into_iter()
                .map(|param| FieldError {
                    field: param.pointer.trim_start_matches('#').trim_start_matches('/').to_string(),
                    message: param.reason.unwrap_or_default(),
                })
                .collect();

            Err(AppError::ValidationFailed(field_errors))
        }
    }
}
```

## Testing Patterns

### Unit Testing Error Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_violations() {
        let json = r#"{"age": 256, "score": 50000}"#;
        let result: Result<UserData> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 2);

        let age_error = &error.invalid_params[0];
        assert_eq!(age_error.name, "age");
        assert_eq!(age_error.pointer, "#/age");
        assert!(age_error.reason.as_ref().unwrap().contains("out of range"));
    }

    #[test]
    fn test_missing_fields() {
        let json = r#"{"name": "Bob"}"#;
        let result: Result<UserData> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.invalid_params.len() > 0);

        let missing_fields: Vec<&str> = error.invalid_params
            .iter()
            .filter(|p| p.reason.as_ref().unwrap().contains("missing"))
            .map(|p| p.name.as_str())
            .collect();

        assert!(missing_fields.contains(&"age"));
    }
}
```

This comprehensive example guide demonstrates the full capabilities of Serdify for handling JSON validation errors in a structured, RFC 7807 compliant manner.
