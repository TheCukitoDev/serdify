use serde::{Deserialize, Serialize};
use serdify::*;

#[derive(Deserialize, Serialize, Debug)]
struct UserData {
    name: String,
    age: u8,
    score: i16,
    balance: u32,
}

#[derive(Deserialize, Debug)]
struct UserWithGrades {
    name: String,
    age: u8,
    score: i16,
    balance: u32,
    grades: Vec<u8>,
}

#[derive(Deserialize, Debug)]
struct NestedData {
    user: UserData,
    metadata: Metadata,
}

#[derive(Deserialize, Debug)]
struct Metadata {
    created_at: String,
    version: u8,
    tags: Vec<String>,
}

fn main() {
    println!("🚀 Serdify - RFC 7807 JSON Error Handling Demo");
    println!("===============================================\n");

    println!("Serdify provides RFC 7807 compliant error handling for JSON deserialization.");
    println!("It collects ALL validation errors in a single pass, providing detailed");
    println!("information about each invalid parameter with JSON pointers.\n");

    // Example 1: Successful parsing
    println!("📋 Example 1: Successful parsing");
    println!("-------------------------------");
    let valid_json = r#"{"name": "Alice", "age": 25, "score": 1500, "balance": 10000}"#;
    let result: Result<UserData> = from_str(valid_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => println!("❌ Error: {error:?}\n"),
    }

    // Example 2: Multiple range violations (shows error collection)
    println!("📋 Example 2: Multiple range violations");
    println!("--------------------------------------");
    println!(
        "JSON: {{\"name\": \"Bob\", \"age\": 256, \"score\": 50000, \"balance\": 5000000000}}"
    );
    let range_error_json = r#"{"name": "Bob", "age": 256, "score": 50000, "balance": 5000000000}"#;
    let result: Result<UserData> = from_str(range_error_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => {
            println!("❌ RFC 7807 Error Response:");
            println!("   Title: {}", error.title);
            println!("   Status: {:?}", error.status);
            println!("   Invalid Parameters ({}):", error.invalid_params.len());
            for param in &error.invalid_params {
                println!(
                    "     • {}: {} (JSON pointer: {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    // Example 3: Missing required fields
    println!("📋 Example 3: Missing required fields");
    println!("------------------------------------");
    println!("JSON: {{\"name\": \"Charlie\"}}");
    let missing_fields_json = r#"{"name": "Charlie"}"#;
    let result: Result<UserData> = from_str(missing_fields_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => {
            println!("❌ RFC 7807 Error Response:");
            println!("   Title: {}", error.title);
            println!("   Invalid Parameters ({}):", error.invalid_params.len());
            for param in &error.invalid_params {
                println!(
                    "     • {}: {} (JSON pointer: {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    // Example 4: JSON syntax error
    println!("📋 Example 4: JSON syntax error");
    println!("-------------------------------");
    println!("JSON: {{\"name\": \"Dave\", \"age\": 30,}} (note the trailing comma)");
    let syntax_error_json = r#"{"name": "Dave", "age": 30,}"#; // trailing comma
    let result: Result<UserData> = from_str(syntax_error_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => {
            println!("❌ RFC 7807 Error Response:");
            println!("   Title: {}", error.title);
            if let Some(detail) = &error.detail {
                println!("   Detail: {detail}");
            }
            println!("   Invalid Parameters: {}", error.invalid_params.len());
            println!();
        }
    }

    // Example 5: Array with range violations (demonstrates JSON pointer for arrays)
    println!("📋 Example 5: Array with range violations");
    println!("----------------------------------------");
    println!(
        "JSON: {{\"name\": \"Eve\", \"age\": 30, \"score\": 1000, \"balance\": 2000, \"grades\": [85, 256, 95, 300]}}"
    );
    let array_json = r#"{"name": "Eve", "age": 30, "score": 1000, "balance": 2000, "grades": [85, 256, 95, 300]}"#;

    let result: Result<UserWithGrades> = from_str(array_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => {
            println!("❌ RFC 7807 Error Response:");
            println!("   Title: {}", error.title);
            println!("   Invalid Parameters ({}):", error.invalid_params.len());
            for param in &error.invalid_params {
                println!(
                    "     • Array index {}: {} (JSON pointer: {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    // Example 6: Nested structures with multiple error types
    println!("📋 Example 6: Nested structures with complex errors");
    println!("--------------------------------------------------");
    let nested_json = r#"{
        "user": {
            "name": "Frank",
            "age": 300,
            "balance": 6000000000
        },
        "metadata": {
            "created_at": "2023-01-01",
            "version": 999,
            "tags": ["user", "premium"]
        }
    }"#;
    println!("JSON: (nested object with range violations)");
    let result: Result<NestedData> = from_str(nested_json);
    match result {
        Result::Ok(value) => println!("✅ Successfully parsed: {value:?}\n"),
        Result::Err(error) => {
            println!("❌ RFC 7807 Error Response:");
            println!("   Title: {}", error.title);
            println!("   Status: {:?}", error.status);
            println!("   Invalid Parameters ({}):", error.invalid_params.len());
            for param in &error.invalid_params {
                println!(
                    "     • {}: {} (JSON pointer: {})",
                    param.name,
                    param.reason.as_ref().unwrap_or(&"No reason".to_string()),
                    param.pointer
                );
            }
            println!();
        }
    }

    println!("🎉 Demo completed!");
    println!("\n📝 Key Features Demonstrated:");
    println!("   • Multiple error collection in a single pass");
    println!("   • Precise JSON pointers for error locations");
    println!("   • RFC 7807 compliant error format");
    println!("   • Detailed type and range information");
    println!("   • Support for nested structures and arrays");
    println!("   • JSON syntax error handling");
}
