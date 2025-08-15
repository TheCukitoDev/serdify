use serde::{Deserialize, Serialize};
use serdify::*;

#[derive(Deserialize, Serialize, Debug)]
struct UserData {
    name: String,
    age: u8,
    score: i16,
    balance: u32,
}

fn main() {
    println!("Serdify - RFC 7807 JSON Error Handling Demo");
    println!("===========================================\n");

    // Example 1: Successful parsing
    println!("1. Successful parsing:");
    let valid_json = r#"{"name": "Alice", "age": 25, "score": 1500, "balance": 10000}"#;
    let result: Result<UserData> = from_str(valid_json);
    match result {
        Result::Ok(value) => println!("✓ Successfully parsed: {:?}\n", value),
        Result::Err(error) => println!("✗ Error: {:?}\n", error),
    }

    // Example 2: Multiple range violations
    println!("2. Multiple range violations:");
    let range_error_json = r#"{"name": "Bob", "age": 256, "score": 50000, "balance": 5000000000}"#;
    let result: Result<UserData> = from_str(range_error_json);
    match result {
        Result::Ok(value) => println!("✓ Successfully parsed: {:?}\n", value),
        Result::Err(error) => {
            println!("✗ RFC 7807 Error Response:");
            println!("Title: {}", error.title);
            println!("Status: {:?}", error.status);
            println!("Invalid Parameters:");
            for param in &error.invalid_params {
                println!(
                    "  - {}: {} (at {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    // Example 3: Missing fields
    println!("3. Missing required fields:");
    let missing_fields_json = r#"{"name": "Charlie"}"#;
    let result: Result<UserData> = from_str(missing_fields_json);
    match result {
        Result::Ok(value) => println!("✓ Successfully parsed: {:?}\n", value),
        Result::Err(error) => {
            println!("✗ RFC 7807 Error Response:");
            println!("Title: {}", error.title);
            println!("Invalid Parameters:");
            for param in &error.invalid_params {
                println!(
                    "  - {}: {} (at {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    // Example 4: JSON syntax error
    println!("4. JSON syntax error:");
    let syntax_error_json = r#"{"name": "Dave", "age": 30,}"#; // trailing comma
    let result: Result<UserData> = from_str(syntax_error_json);
    match result {
        Result::Ok(value) => println!("✓ Successfully parsed: {:?}\n", value),
        Result::Err(error) => {
            println!("✗ RFC 7807 Error Response:");
            println!("Title: {}", error.title);
            if let Some(detail) = &error.detail {
                println!("Detail: {}", detail);
            }
            println!("Invalid Parameters: {}", error.invalid_params.len());
            println!();
        }
    }

    // Example 5: Array with range violations
    println!("5. Array with range violations:");
    let array_json = r#"{"name": "Eve", "age": 30, "score": 1000, "balance": 2000, "grades": [85, 256, 95, 300]}"#;
    #[derive(Deserialize, Debug)]
    struct UserWithGrades {
        name: String,
        age: u8,
        score: i16,
        balance: u32,
        grades: Vec<u8>,
    }

    let result: Result<UserWithGrades> = from_str(array_json);
    match result {
        Result::Ok(value) => println!("✓ Successfully parsed: {:?}\n", value),
        Result::Err(error) => {
            println!("✗ RFC 7807 Error Response:");
            println!("Title: {}", error.title);
            println!("Invalid Parameters:");
            for param in &error.invalid_params {
                println!(
                    "  - {}: {} (at {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    println!("Demo completed! 🎉");
}
