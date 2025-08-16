#[cfg(test)]
mod debug_optional_test {
    
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct OptionalStruct {
        required_field: String,
        optional_field: Option<String>,
        optional_number: Option<i32>,
    }

    #[test]
    fn debug_optional_fields() {
        let json_with_optional = r#"{
            "required_field": "test",
            "optional_field": "present",
            "optional_number": 42
        }"#;

        // Test with serde_json
        let serde_result: std::result::Result<OptionalStruct, _> =
            serde_json::from_str(json_with_optional);
        println!("serde_json result: {serde_result:?}");

        // Test with our implementation
        let our_result: crate::Result<OptionalStruct> = crate::from_str(json_with_optional);
        println!("our result: {our_result:?}");

        if let crate::Result::Err(ref err) = our_result {
            println!("Error details: {err:?}");
            for param in &err.invalid_params {
                println!("Invalid param: {param:?}");
            }
        }
    }
}
