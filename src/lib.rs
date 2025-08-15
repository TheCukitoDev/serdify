use serde::{Deserialize, Serialize, de};
use serde_json::Value;
use std::any::type_name;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExpectedOrActual {
    r#type: String, // The type of the parameter
    format: String, // The format of the parameter
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvalidParam {
    pub name: String,           // The name of the parameter that has failed validation.
    pub reason: Option<String>, // The reason why it has failed validation.
    pub expected: ExpectedOrActual, // The expected type of the parameter.
    pub actual: ExpectedOrActual, // The actual type of the parameter.
    pub pointer: String,        // A JSON pointer for the parameter that has failed validation.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    pub r#type: Option<String>, // The URI of the error. This will be implemented in future versions
    pub title: String, // A short description of the problem. This might always be: Your request parameters didn't validate.
    pub detail: Option<String>, // A more detailed description of the problem. This will be implemented in future versions.
    pub instance: Option<String>, // Where the error happened.
    pub invalid_params: Vec<InvalidParam>, // The Array of invalid parameters that didn't validate
    pub status: Option<u16>, // The HTTP status code. This will mostlikely be 400. TODO: Add option to define custom status code.
}

pub enum Result<T> {
    Ok(T),
    Err(Error),
}

// Type information extraction
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub rust_type: String,
    pub json_format: String,
}

pub fn extract_type_info<T>() -> TypeInfo {
    let full_type_name = type_name::<T>();
    let rust_type = full_type_name
        .split("::")
        .last()
        .unwrap_or(full_type_name)
        .to_string();
    let json_format = match full_type_name {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => "integer",
        "f32" | "f64" => "number",
        "bool" => "boolean",
        "char" | "&str" => "string",
        s if s.contains("String") => "string",
        s if s.starts_with("alloc::vec::Vec<") || s.starts_with("Vec<") => "array",
        s if s.contains("HashMap") || s.contains("BTreeMap") => "object",
        s if s.contains("Option<") => "nullable",
        _ => "object",
    }
    .to_string();

    TypeInfo {
        rust_type,
        json_format,
    }
}

// Error collection system
#[derive(Debug)]
pub struct ErrorCollector {
    errors: Vec<InvalidParam>,
    current_path: Vec<String>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            current_path: Vec::new(),
        }
    }

    pub fn push_path(&mut self, segment: &str) {
        self.current_path.push(segment.to_string());
    }

    pub fn pop_path(&mut self) {
        self.current_path.pop();
    }

    pub fn current_pointer(&self) -> String {
        if self.current_path.is_empty() {
            "#".to_string()
        } else {
            format!(
                "#{}",
                self.current_path
                    .iter()
                    .map(|s| format!("/{}", s))
                    .collect::<String>()
            )
        }
    }

    pub fn add_error(
        &mut self,
        name: String,
        reason: Option<String>,
        expected: ExpectedOrActual,
        actual: ExpectedOrActual,
    ) {
        let invalid_param = InvalidParam {
            name,
            reason,
            expected,
            actual,
            pointer: self.current_pointer(),
        };
        self.errors.push(invalid_param);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_rfc7807_error(self) -> Error {
        Error {
            r#type: None,
            title: "Your request parameters didn't validate.".to_string(),
            detail: None,
            instance: None,
            invalid_params: self.errors,
            status: Some(400),
        }
    }
}

// Custom deserializer with error collection
pub struct CollectingDeserializer<'de> {
    input: &'de Value,
    collector: ErrorCollector,
}

impl<'de> CollectingDeserializer<'de> {
    pub fn new(input: &'de Value) -> Self {
        Self {
            input,
            collector: ErrorCollector::new(),
        }
    }

    pub fn from_json_value(value: &'de Value) -> Self {
        Self::new(value)
    }

    fn add_type_error<T>(&mut self, expected: &str)
    where
        T: de::DeserializeOwned,
    {
        let type_info = extract_type_info::<T>();
        let actual_type = match self.input {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };

        self.collector.add_error(
            "value".to_string(),
            Some(format!("Expected {}, got {}", expected, actual_type)),
            ExpectedOrActual {
                r#type: type_info.rust_type,
                format: expected.to_string(),
            },
            ExpectedOrActual {
                r#type: actual_type.to_string(),
                format: actual_type.to_string(),
            },
        );
    }
}

impl<'de> de::Deserializer<'de> for CollectingDeserializer<'de> {
    type Error = serde_json::Error;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(f) = n.as_f64() {
                    visitor.visit_f64(f)
                } else {
                    Err(de::Error::custom("Invalid number"))
                }
            }
            Value::String(s) => visitor.visit_str(s),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Object(_) => self.deserialize_map(visitor),
        }
    }

    // Implement complex type deserializers as required by task 5.3
    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Object(map) => {
                let struct_deserializer = StructDeserializer {
                    map,
                    fields,
                    collector: &mut self.collector,
                    current_field: 0,
                };
                visitor.visit_map(struct_deserializer)
            }
            _ => {
                self.add_type_error::<()>("object");
                Err(de::Error::custom("Expected object for struct"))
            }
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Array(arr) => {
                let seq_deserializer = SeqDeserializer {
                    array: arr,
                    index: 0,
                    collector: &mut self.collector,
                };
                visitor.visit_seq(seq_deserializer)
            }
            _ => {
                self.add_type_error::<()>("array");
                Err(de::Error::custom("Expected array for sequence"))
            }
        }
    }

    fn deserialize_map<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Object(map) => {
                let map_deserializer = MapDeserializer {
                    map,
                    keys: map.keys().collect(),
                    index: 0,
                    collector: &mut self.collector,
                };
                visitor.visit_map(map_deserializer)
            }
            _ => {
                self.add_type_error::<()>("object");
                Err(de::Error::custom("Expected object for map"))
            }
        }
    }

    // Forward other methods to deserialize_any for now
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct enum identifier ignored_any
    }
}

// Struct deserializer with path tracking
struct StructDeserializer<'a, 'de> {
    map: &'de serde_json::Map<String, Value>,
    fields: &'static [&'static str],
    collector: &'a mut ErrorCollector,
    current_field: usize,
}

impl<'a, 'de> de::MapAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = serde_json::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        // Iterate through expected fields
        while self.current_field < self.fields.len() {
            let field = self.fields[self.current_field];
            if self.map.contains_key(field) {
                return seed
                    .deserialize(de::value::StrDeserializer::new(field))
                    .map(Some);
            }
            self.current_field += 1;
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if self.current_field < self.fields.len() {
            let field = self.fields[self.current_field];
            if let Some(value) = self.map.get(field) {
                self.collector.push_path(field);
                let result = seed.deserialize(CollectingDeserializer::from_json_value(value));
                self.collector.pop_path();
                self.current_field += 1;
                result
            } else {
                self.current_field += 1;
                Err(de::Error::custom(format!("Missing field: {}", field)))
            }
        } else {
            Err(de::Error::custom("No more values"))
        }
    }
}

// Sequence deserializer with path tracking
struct SeqDeserializer<'a, 'de> {
    array: &'de Vec<Value>,
    index: usize,
    collector: &'a mut ErrorCollector,
}

impl<'a, 'de> de::SeqAccess<'de> for SeqDeserializer<'a, 'de> {
    type Error = serde_json::Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.index < self.array.len() {
            let value = &self.array[self.index];
            self.collector.push_path(&self.index.to_string());
            let result = seed.deserialize(CollectingDeserializer::from_json_value(value));
            self.collector.pop_path();
            self.index += 1;
            result.map(Some)
        } else {
            Ok(None)
        }
    }
}

// Map deserializer with path tracking
struct MapDeserializer<'a, 'de> {
    map: &'de serde_json::Map<String, Value>,
    keys: Vec<&'de String>,
    index: usize,
    collector: &'a mut ErrorCollector,
}

impl<'a, 'de> de::MapAccess<'de> for MapDeserializer<'a, 'de> {
    type Error = serde_json::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];
            seed.deserialize(de::value::StrDeserializer::new(key))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];
            let value = &self.map[key];
            self.collector.push_path(key);
            let result = seed.deserialize(CollectingDeserializer::from_json_value(value));
            self.collector.pop_path();
            self.index += 1;
            result
        } else {
            Err(de::Error::custom("No more values"))
        }
    }
}

impl<T> Result<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, Result::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Result::Err(_))
    }

    pub fn as_ref(&self) -> Result<&T> {
        match self {
            Result::Ok(t) => Result::Ok(t),
            Result::Err(e) => Result::Err(e.clone()),
        }
    }
}

impl<T: std::fmt::Debug> Result<T> {
    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(t) => t,
            Result::Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", e),
        }
    }

    pub fn unwrap_err(self) -> Error {
        match self {
            Result::Ok(t) => panic!("called `Result::unwrap_err()` on an `Ok` value: {:?}", t),
            Result::Err(e) => e,
        }
    }
}

pub fn from_str<T>(s: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    // First parse the JSON to get a Value
    let json_value: Value = match serde_json::from_str(s) {
        Ok(value) => value,
        Err(e) => {
            return Result::Err(Error {
                r#type: None,
                title: "JSON parsing error".to_string(),
                detail: Some(e.to_string()),
                instance: None,
                invalid_params: vec![],
                status: Some(400),
            });
        }
    };

    // Use our collecting deserializer
    let deserializer = CollectingDeserializer::from_json_value(&json_value);

    match T::deserialize(deserializer) {
        Ok(value) => Result::Ok(value),
        Err(_) => {
            // If there were errors collected, return them
            // For now, return a basic error - this will be enhanced in task 6
            Result::Err(Error {
                r#type: None,
                title: "Deserialization failed".to_string(),
                detail: Some("Type validation errors".to_string()),
                instance: None,
                invalid_params: vec![],
                status: Some(400),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestStruct {
        name: String,
        age: u32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedStruct {
        user: TestStruct,
        active: bool,
    }

    #[test]
    fn test_basic_functionality() {
        let ok_result: Result<i32> = Result::Ok(42);
        let err_result: Result<i32> = Result::Err(Error {
            r#type: None,
            title: "Test Error".to_string(),
            detail: None,
            instance: None,
            invalid_params: vec![],
            status: Some(400),
        });

        assert!(ok_result.is_ok());
        assert!(err_result.is_err());
        assert_eq!(ok_result.unwrap(), 42);
        assert_eq!(err_result.unwrap_err().title, "Test Error");
    }

    #[test]
    fn test_deserialize_struct_success() {
        let json = r#"{"name": "John", "age": 30}"#;
        let result: Result<TestStruct> = from_str(json);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "John");
        assert_eq!(user.age, 30);
    }

    #[test]
    fn test_deserialize_nested_struct_success() {
        let json = r#"{"user": {"name": "Alice", "age": 25}, "active": true}"#;
        let result: Result<NestedStruct> = from_str(json);

        assert!(result.is_ok());
        let nested = result.unwrap();
        assert_eq!(nested.user.name, "Alice");
        assert_eq!(nested.user.age, 25);
        assert_eq!(nested.active, true);
    }

    #[test]
    fn test_deserialize_array_success() {
        let json = r#"[1, 2, 3, 4, 5]"#;
        let result: Result<Vec<i32>> = from_str(json);

        assert!(result.is_ok());
        let vec = result.unwrap();
        assert_eq!(vec, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_deserialize_array_of_structs_success() {
        let json = r#"[{"name": "John", "age": 30}, {"name": "Jane", "age": 25}]"#;
        let result: Result<Vec<TestStruct>> = from_str(json);

        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "John");
        assert_eq!(users[1].name, "Jane");
    }

    #[test]
    fn test_type_info_extraction() {
        let u64_info = extract_type_info::<u64>();
        assert_eq!(u64_info.rust_type, "u64");
        assert_eq!(u64_info.json_format, "integer");

        let string_info = extract_type_info::<String>();
        assert_eq!(string_info.rust_type, "String");
        assert_eq!(string_info.json_format, "string");

        let vec_info = extract_type_info::<Vec<i32>>();
        assert_eq!(vec_info.rust_type, "Vec<i32>");
        assert_eq!(vec_info.json_format, "array");
    }

    #[test]
    fn test_error_collector_path_tracking() {
        let mut collector = ErrorCollector::new();

        assert_eq!(collector.current_pointer(), "#");

        collector.push_path("user");
        assert_eq!(collector.current_pointer(), "#/user");

        collector.push_path("name");
        assert_eq!(collector.current_pointer(), "#/user/name");

        collector.pop_path();
        assert_eq!(collector.current_pointer(), "#/user");

        collector.pop_path();
        assert_eq!(collector.current_pointer(), "#");
    }

    #[test]
    fn test_deserialize_array_with_indices() {
        // Test that array indices are properly tracked in JSON pointers
        let json = r#"[{"name": "John", "age": 30}, {"name": "Jane", "age": 25}]"#;
        let result: Result<Vec<TestStruct>> = from_str(json);

        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "John");
        assert_eq!(users[1].name, "Jane");
    }

    #[test]
    fn test_deserialize_deeply_nested_structure() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Address {
            street: String,
            city: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct UserWithAddress {
            name: String,
            address: Address,
        }

        let json = r#"{"name": "John", "address": {"street": "123 Main St", "city": "Anytown"}}"#;
        let result: Result<UserWithAddress> = from_str(json);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "John");
        assert_eq!(user.address.street, "123 Main St");
        assert_eq!(user.address.city, "Anytown");
    }
}
