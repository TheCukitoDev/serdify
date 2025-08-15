use serdify::*;

fn main() {
    println!("Serdify - RFC 7807 JSON Error Handling");

    // Example usage
    let result: Result<i32> = from_str("42");
    match result {
        Result::Ok(value) => println!("Successfully parsed: {}", value),
        Result::Err(error) => println!("Error: {:?}", error),
    }
}
