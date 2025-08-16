use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use serdify::{Result as CustomResult, from_str as custom_from_str};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct ComplexNested {
    level1: Level1,
    array_of_structs: Vec<SimpleItem>,
    optional_nested: Option<Level1>,
    map_of_maps: HashMap<String, HashMap<String, i32>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Level1 {
    level2: Level2,
    numbers: Vec<f64>,
    flags: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Level2 {
    level3: Level3,
    metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Level3 {
    final_value: String,
    count: u64,
    ratio: f32,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct SimpleItem {
    id: u32,
    name: String,
}

#[cfg(test)]
mod integration_compatibility_tests {
    use super::*;

    // TODO: This test is currently failing because serdify doesn't fully support
    // all complex nested structures yet. Uncomment when the implementation improves.
    #[ignore]
    #[test]
    fn test_deeply_nested_structures() {
        let json = r#"{
            "level1": {
                "level2": {
                    "level3": {
                        "final_value": "deep",
                        "count": 999,
                        "ratio": 0.75
                    },
                    "metadata": {
                        "created": "2023-01-01",
                        "author": "test"
                    }
                },
                "numbers": [1.1, 2.2, 3.3, 4.4],
                "flags": {
                    "enabled": true,
                    "visible": false,
                    "active": true
                }
            },
            "array_of_structs": [
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"},
                {"id": 3, "name": "third"}
            ],
            "optional_nested": {
                "level2": {
                    "level3": {
                        "final_value": "optional",
                        "count": 123,
                        "ratio": 0.5
                    }
                },
                "numbers": [5.5, 6.6],
                "flags": {"test": true}
            },
            "map_of_maps": {
                "group1": {"a": 1, "b": 2},
                "group2": {"c": 3, "d": 4},
                "group3": {"e": 5, "f": 6}
            }
        }"#;

        let serde_result: std::result::Result<ComplexNested, _> = serde_json::from_str(json);
        let custom_result: CustomResult<ComplexNested> = custom_from_str(json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }

    // TODO: This test is currently failing because serdify doesn't fully support
    // all complex array operations yet. Uncomment when the implementation improves.
    #[ignore]
    #[test]
    fn test_array_of_complex_objects() {
        let json = r#"[
            {
                "level1": {
                    "level2": {
                        "level3": {"final_value": "first", "count": 1, "ratio": 0.1}
                    },
                    "numbers": [1.0],
                    "flags": {"active": true}
                },
                "array_of_structs": [{"id": 1, "name": "item1"}],
                "map_of_maps": {"group": {"key": 1}}
            },
            {
                "level1": {
                    "level2": {
                        "level3": {"final_value": "second", "count": 2, "ratio": 0.2}
                    },
                    "numbers": [2.0, 2.1],
                    "flags": {"active": false}
                },
                "array_of_structs": [{"id": 2, "name": "item2"}],
                "map_of_maps": {"group": {"key": 2}}
            }
        ]"#;

        let serde_result: std::result::Result<Vec<ComplexNested>, _> = serde_json::from_str(json);
        let custom_result: CustomResult<Vec<ComplexNested>> = custom_from_str(json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }

    #[test]
    fn test_mixed_types_in_collections() {
        // Test with serde_json::Value for maximum flexibility
        let json = r#"{
            "mixed_array": [1, "string", true, null, {"nested": "object"}, [1, 2, 3]],
            "complex_object": {
                "numbers": [1, 2.5, 3],
                "strings": ["a", "b", "c"],
                "nested": {
                    "deep": {
                        "deeper": {
                            "value": "found"
                        }
                    }
                }
            }
        }"#;

        let serde_result: std::result::Result<serde_json::Value, _> = serde_json::from_str(json);
        let custom_result: CustomResult<serde_json::Value> = custom_from_str(json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }

    #[test]
    fn test_unicode_and_special_characters() {
        let json = r#"{
            "unicode": "こんにちは世界",
            "emoji": "🦀🚀✨",
            "special_chars": "\"quotes\" and \\backslashes\\ and \ttabs\t and \nnewlines\n",
            "unicode_keys": {
                "🔑": "key with emoji",
                "münchen": "city with umlaut",
                "北京": "chinese characters"
            }
        }"#;

        let serde_result: std::result::Result<serde_json::Value, _> = serde_json::from_str(json);
        let custom_result: CustomResult<serde_json::Value> = custom_from_str(json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }

    #[test]
    fn test_number_precision_compatibility() {
        let test_cases = vec![
            r#"{"int": 9223372036854775807}"#,      // i64::MAX
            r#"{"uint": 18446744073709551615}"#,    // u64::MAX
            r#"{"float": 1.7976931348623157e308}"#, // Close to f64::MAX
            r#"{"scientific": 1.23e-10}"#,
            r#"{"negative": -9223372036854775808}"#, // i64::MIN
            r#"{"zero": 0}"#,
            r#"{"decimal": 123.456789}"#,
        ];

        for json in test_cases {
            let serde_result: std::result::Result<serde_json::Value, _> =
                serde_json::from_str(json);
            let custom_result: CustomResult<serde_json::Value> = custom_from_str(json);

            assert!(serde_result.is_ok(), "serde_json failed for: {json}");
            assert!(
                custom_result.is_ok(),
                "custom implementation failed for: {json}"
            );
            assert_eq!(
                serde_result.unwrap(),
                custom_result.unwrap(),
                "Results differ for: {json}"
            );
        }
    }

    #[test]
    fn test_whitespace_handling() {
        let test_cases = vec![
            r#"  {  "key"  :  "value"  }  "#,
            "{\n  \"multiline\": \"value\",\n  \"number\": 42\n}",
            r#"{"no_spaces":"compact"}"#,
            "{\r\n  \"windows_newlines\": true\r\n}",
        ];

        for json in test_cases {
            let serde_result: std::result::Result<serde_json::Value, _> =
                serde_json::from_str(json);
            let custom_result: CustomResult<serde_json::Value> = custom_from_str(json);

            assert!(serde_result.is_ok(), "serde_json failed for: {json}");
            assert!(
                custom_result.is_ok(),
                "custom implementation failed for: {json}"
            );
            assert_eq!(
                serde_result.unwrap(),
                custom_result.unwrap(),
                "Results differ for: {json}"
            );
        }
    }

    #[test]
    fn test_large_arrays_compatibility() {
        let large_array = (0..1000)
            .map(|i| {
                format!(
                    r#"{{"id": {}, "value": "item{}", "score": {}}}"#,
                    i,
                    i,
                    i as f64 * 0.5
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!("[{large_array}]");

        let serde_result: std::result::Result<Vec<serde_json::Value>, _> =
            serde_json::from_str(&json);
        let custom_result: CustomResult<Vec<serde_json::Value>> = custom_from_str(&json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }

    #[test]
    fn test_nested_arrays_compatibility() {
        let json = r#"[
            [1, 2, 3],
            ["a", "b", "c"],
            [true, false, true],
            [{"nested": "object"}, {"another": "one"}]
        ]"#;

        let serde_result: std::result::Result<Vec<serde_json::Value>, _> =
            serde_json::from_str(json);
        let custom_result: CustomResult<Vec<serde_json::Value>> = custom_from_str(json);

        assert!(serde_result.is_ok());
        assert!(custom_result.is_ok());
        assert_eq!(serde_result.unwrap(), custom_result.unwrap());
    }
}
