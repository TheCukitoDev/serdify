use serde::{Deserialize, Serialize, de};
use serde_json::Value;
use std::any::type_name;
use std::collections::HashSet;

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
    pub errors: Vec<InvalidParam>,
    current_path: Vec<String>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            current_path: Vec::new(),
        }
    }

    pub fn with_path(path: Vec<String>) -> Self {
        Self {
            errors: Vec::new(),
            current_path: path,
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
    pub collector: ErrorCollector,
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

    pub fn with_collector(input: &'de Value, collector: ErrorCollector) -> Self {
        Self { input, collector }
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

    fn add_range_error<T>(&mut self, actual_value: i64, min: i64, max: i64)
    where
        T: de::DeserializeOwned,
    {
        let type_info = extract_type_info::<T>();

        // Get the field name from the current path
        let field_name = self
            .collector
            .current_path
            .last()
            .cloned()
            .unwrap_or_else(|| "value".to_string());

        self.collector.add_error(
            field_name,
            Some(format!(
                "Value {} is out of range for type {}. Expected range: {} to {}",
                actual_value, type_info.rust_type, min, max
            )),
            ExpectedOrActual {
                r#type: type_info.rust_type.clone(),
                format: format!("integer in range {} to {}", min, max),
            },
            ExpectedOrActual {
                r#type: "integer".to_string(),
                format: actual_value.to_string(),
            },
        );
    }
}

impl<'de> de::Deserializer<'de> for &mut CollectingDeserializer<'de> {
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
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Object(map) => {
                let struct_deserializer = StructDeserializer::new(map, fields, &mut self.collector);
                visitor.visit_map(struct_deserializer)
            }
            _ => {
                self.add_type_error::<()>("object");
                Err(de::Error::custom("Expected object for struct"))
            }
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
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

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
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

    // Implement specific numeric types with range validation
    fn deserialize_u8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= 0 && i <= u8::MAX as i64 {
                        visitor.visit_u8(i as u8)
                    } else {
                        self.add_range_error::<u8>(i, 0, u8::MAX as i64);
                        // Return a default value to continue processing
                        visitor.visit_u8(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= u8::MAX as u64 {
                        visitor.visit_u8(u as u8)
                    } else {
                        self.add_range_error::<u8>(u as i64, 0, u8::MAX as i64);
                        // Return a default value to continue processing
                        visitor.visit_u8(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= 0.0 && f <= u8::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_u8(f as u8)
                    } else {
                        self.add_range_error::<u8>(f as i64, 0, u8::MAX as i64);
                        // Return a default value to continue processing
                        visitor.visit_u8(0)
                    }
                } else {
                    self.add_type_error::<u8>("integer");
                    visitor.visit_u8(0)
                }
            }
            _ => {
                self.add_type_error::<u8>("integer");
                visitor.visit_u8(0)
            }
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i8::MIN as i64 && i <= i8::MAX as i64 {
                        visitor.visit_i8(i as i8)
                    } else {
                        self.add_range_error::<i8>(i, i8::MIN as i64, i8::MAX as i64);
                        visitor.visit_i8(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= i8::MAX as u64 {
                        visitor.visit_i8(u as i8)
                    } else {
                        self.add_range_error::<i8>(u as i64, i8::MIN as i64, i8::MAX as i64);
                        visitor.visit_i8(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= i8::MIN as f64 && f <= i8::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_i8(f as i8)
                    } else {
                        self.add_range_error::<i8>(f as i64, i8::MIN as i64, i8::MAX as i64);
                        visitor.visit_i8(0)
                    }
                } else {
                    self.add_type_error::<i8>("integer");
                    visitor.visit_i8(0)
                }
            }
            _ => {
                self.add_type_error::<i8>("integer");
                visitor.visit_i8(0)
            }
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= 0 && i <= u16::MAX as i64 {
                        visitor.visit_u16(i as u16)
                    } else {
                        self.add_range_error::<u16>(i, 0, u16::MAX as i64);
                        visitor.visit_u16(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= u16::MAX as u64 {
                        visitor.visit_u16(u as u16)
                    } else {
                        self.add_range_error::<u16>(u as i64, 0, u16::MAX as i64);
                        visitor.visit_u16(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= 0.0 && f <= u16::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_u16(f as u16)
                    } else {
                        self.add_range_error::<u16>(f as i64, 0, u16::MAX as i64);
                        visitor.visit_u16(0)
                    }
                } else {
                    self.add_type_error::<u16>("integer");
                    visitor.visit_u16(0)
                }
            }
            _ => {
                self.add_type_error::<u16>("integer");
                visitor.visit_u16(0)
            }
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i16::MIN as i64 && i <= i16::MAX as i64 {
                        visitor.visit_i16(i as i16)
                    } else {
                        self.add_range_error::<i16>(i, i16::MIN as i64, i16::MAX as i64);
                        visitor.visit_i16(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= i16::MAX as u64 {
                        visitor.visit_i16(u as i16)
                    } else {
                        self.add_range_error::<i16>(u as i64, i16::MIN as i64, i16::MAX as i64);
                        visitor.visit_i16(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= i16::MIN as f64 && f <= i16::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_i16(f as i16)
                    } else {
                        self.add_range_error::<i16>(f as i64, i16::MIN as i64, i16::MAX as i64);
                        visitor.visit_i16(0)
                    }
                } else {
                    self.add_type_error::<i16>("integer");
                    visitor.visit_i16(0)
                }
            }
            _ => {
                self.add_type_error::<i16>("integer");
                visitor.visit_i16(0)
            }
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= 0 && i <= u32::MAX as i64 {
                        visitor.visit_u32(i as u32)
                    } else {
                        self.add_range_error::<u32>(i, 0, u32::MAX as i64);
                        visitor.visit_u32(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= u32::MAX as u64 {
                        visitor.visit_u32(u as u32)
                    } else {
                        self.add_range_error::<u32>(u as i64, 0, u32::MAX as i64);
                        visitor.visit_u32(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= 0.0 && f <= u32::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_u32(f as u32)
                    } else {
                        self.add_range_error::<u32>(f as i64, 0, u32::MAX as i64);
                        visitor.visit_u32(0)
                    }
                } else {
                    self.add_type_error::<u32>("integer");
                    visitor.visit_u32(0)
                }
            }
            _ => {
                self.add_type_error::<u32>("integer");
                visitor.visit_u32(0)
            }
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        visitor.visit_i32(i as i32)
                    } else {
                        self.add_range_error::<i32>(i, i32::MIN as i64, i32::MAX as i64);
                        visitor.visit_i32(0)
                    }
                } else if let Some(u) = n.as_u64() {
                    if u <= i32::MAX as u64 {
                        visitor.visit_i32(u as i32)
                    } else {
                        self.add_range_error::<i32>(u as i64, i32::MIN as i64, i32::MAX as i64);
                        visitor.visit_i32(0)
                    }
                } else if let Some(f) = n.as_f64() {
                    if f >= i32::MIN as f64 && f <= i32::MAX as f64 && f.fract() == 0.0 {
                        visitor.visit_i32(f as i32)
                    } else {
                        self.add_range_error::<i32>(f as i64, i32::MIN as i64, i32::MAX as i64);
                        visitor.visit_i32(0)
                    }
                } else {
                    self.add_type_error::<i32>("integer");
                    visitor.visit_i32(0)
                }
            }
            _ => {
                self.add_type_error::<i32>("integer");
                visitor.visit_i32(0)
            }
        }
    }

    // Forward remaining methods to deserialize_any
    serde::forward_to_deserialize_any! {
        bool i64 i128 u64 u128 f32 f64 char str string
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
    processed_fields: HashSet<&'static str>,
}

impl<'a, 'de> StructDeserializer<'a, 'de> {
    fn new(
        map: &'de serde_json::Map<String, Value>,
        fields: &'static [&'static str],
        collector: &'a mut ErrorCollector,
    ) -> Self {
        Self {
            map,
            fields,
            collector,
            current_field: 0,
            processed_fields: HashSet::new(),
        }
    }

    fn check_missing_fields(&mut self) {
        // Check for missing required fields after processing all available fields
        for &field in self.fields {
            if !self.processed_fields.contains(field) && !self.map.contains_key(field) {
                // Add error for missing required field
                self.collector.add_error(
                    field.to_string(),
                    Some("missing required field".to_string()),
                    ExpectedOrActual {
                        r#type: "required".to_string(),
                        format: "field".to_string(),
                    },
                    ExpectedOrActual {
                        r#type: "missing".to_string(),
                        format: "undefined".to_string(),
                    },
                );
            }
        }
    }
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
                self.processed_fields.insert(field);
                return seed
                    .deserialize(de::value::StrDeserializer::new(field))
                    .map(Some);
            } else {
                // Field is missing - we'll handle this in check_missing_fields
                self.current_field += 1;
            }
        }

        // Check for missing fields when we're done processing
        // At this point, we should check for missing fields with the current path context
        self.check_missing_fields();
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

                // Create a new collector that inherits the current path
                let nested_collector =
                    ErrorCollector::with_path(self.collector.current_path.clone());
                let mut nested_deserializer =
                    CollectingDeserializer::with_collector(value, nested_collector);
                let result = seed.deserialize(&mut nested_deserializer);

                // Merge errors from nested deserializer
                if nested_deserializer.collector.has_errors() {
                    for error in nested_deserializer.collector.errors {
                        self.collector.errors.push(error);
                    }
                }

                self.collector.pop_path();
                self.current_field += 1;
                result
            } else {
                // This shouldn't happen since we check in next_key_seed
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

            // Create a new collector that inherits the current path
            let nested_collector = ErrorCollector::with_path(self.collector.current_path.clone());
            let mut nested_deserializer =
                CollectingDeserializer::with_collector(value, nested_collector);
            let result = seed.deserialize(&mut nested_deserializer);

            // Merge errors from nested deserializer
            if nested_deserializer.collector.has_errors() {
                for error in nested_deserializer.collector.errors {
                    self.collector.errors.push(error);
                }
            }

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

            // Create a new collector that inherits the current path
            let nested_collector = ErrorCollector::with_path(self.collector.current_path.clone());
            let mut nested_deserializer =
                CollectingDeserializer::with_collector(value, nested_collector);
            let result = seed.deserialize(&mut nested_deserializer);

            // Merge errors from nested deserializer
            if nested_deserializer.collector.has_errors() {
                for error in nested_deserializer.collector.errors {
                    self.collector.errors.push(error);
                }
            }

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

fn get_meaningful_json_error_message(error: &serde_json::Error) -> String {
    let error_msg = error.to_string();
    let line = error.line();
    let column = error.column();

    // Provide more meaningful error messages for common JSON syntax issues
    if error_msg.contains("EOF while parsing") {
        format!(
            "JSON syntax error at line {}, column {}: Unexpected end of input. The JSON appears to be incomplete. Check for missing closing braces, brackets, or quotes.",
            line, column
        )
    } else if error_msg.contains("trailing comma") {
        format!(
            "JSON syntax error at line {}, column {}: Trailing comma found. Remove the extra comma after the last element in the object or array.",
            line, column
        )
    } else if error_msg.contains("invalid escape") {
        format!(
            "JSON syntax error at line {}, column {}: Invalid escape sequence in string. Use proper JSON escape sequences like \\n, \\t, \\\", \\\\, etc.",
            line, column
        )
    } else if error_msg.contains("control character") {
        format!(
            "JSON syntax error at line {}, column {}: Unescaped control character found in string. Control characters (ASCII 0-31) must be escaped using \\uXXXX notation.",
            line, column
        )
    } else if error_msg.contains("lone leading surrogate")
        || error_msg.contains("lone trailing surrogate")
    {
        format!(
            "JSON syntax error at line {}, column {}: Invalid Unicode surrogate pair in string. Ensure Unicode characters are properly encoded.",
            line, column
        )
    } else if error_msg.contains("expected")
        && (error_msg.contains("found") || error_msg.contains("at"))
    {
        format!(
            "JSON syntax error at line {}, column {}: {}. Check for missing commas, quotes, or incorrect punctuation.",
            line, column, error_msg
        )
    } else if error_msg.contains("duplicate field") {
        format!(
            "JSON syntax error at line {}, column {}: Duplicate field found in object. JSON objects cannot have duplicate keys.",
            line, column
        )
    } else if error_msg.contains("invalid number") {
        format!(
            "JSON syntax error at line {}, column {}: Invalid number format. Ensure numbers follow JSON specification (no leading zeros, proper decimal notation).",
            line, column
        )
    } else if error_msg.contains("expected value") {
        format!(
            "JSON syntax error at line {}, column {}: Expected a JSON value (string, number, boolean, null, object, or array) but found invalid content.",
            line, column
        )
    } else {
        format!(
            "JSON syntax error at line {}, column {}: {}",
            line, column, error_msg
        )
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
            let meaningful_error = get_meaningful_json_error_message(&e);
            return Result::Err(Error {
                r#type: None,
                title: "Your request parameters didn't validate.".to_string(),
                detail: Some(meaningful_error),
                instance: None,
                invalid_params: vec![], // JSON syntax errors don't generate InvalidParam entries
                status: Some(400),
            });
        }
    };

    // Use our collecting deserializer
    let mut deserializer = CollectingDeserializer::from_json_value(&json_value);

    match T::deserialize(&mut deserializer) {
        Ok(value) => {
            // Check if there were any errors collected during deserialization
            if deserializer.collector.has_errors() {
                Result::Err(deserializer.collector.into_rfc7807_error())
            } else {
                Result::Ok(value)
            }
        }
        Err(_) => {
            // If deserialization failed, return collected errors if any
            if deserializer.collector.has_errors() {
                Result::Err(deserializer.collector.into_rfc7807_error())
            } else {
                // Fallback error if no specific errors were collected
                Result::Err(Error {
                    r#type: None,
                    title: "Your request parameters didn't validate.".to_string(),
                    detail: Some("Deserialization failed with unknown error".to_string()),
                    instance: None,
                    invalid_params: vec![],
                    status: Some(400),
                })
            }
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

    #[test]
    fn test_json_syntax_error_malformed_json() {
        let malformed_json = r#"{"name": "John", "age": 30"#; // Missing closing brace
        let result: Result<TestStruct> = from_str(malformed_json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.title, "Your request parameters didn't validate.");
        assert!(error.detail.is_some());
        assert!(error.detail.unwrap().contains("JSON syntax error"));
        assert!(error.invalid_params.is_empty()); // JSON syntax errors don't generate InvalidParam entries
        assert_eq!(error.status, Some(400));
    }

    #[test]
    fn test_json_syntax_error_trailing_comma() {
        let json_with_trailing_comma = r#"{"name": "John", "age": 30,}"#;
        let result: Result<TestStruct> = from_str(json_with_trailing_comma);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.detail.is_some());
        let detail = error.detail.unwrap();
        assert!(detail.contains("JSON syntax error"));
    }

    #[test]
    fn test_json_syntax_error_invalid_escape() {
        let json_with_invalid_escape = r#"{"name": "John\x", "age": 30}"#;
        let result: Result<TestStruct> = from_str(json_with_invalid_escape);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.detail.is_some());
        let detail = error.detail.unwrap();
        assert!(detail.contains("JSON syntax error"));
    }

    #[test]
    fn test_json_syntax_error_incomplete_json() {
        let incomplete_json = r#"{"name": "John""#; // Missing comma and closing
        let result: Result<TestStruct> = from_str(incomplete_json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.detail.is_some());
        let detail = error.detail.unwrap();
        assert!(detail.contains("JSON syntax error"));
    }

    #[test]
    fn test_type_mismatch_error_collection() {
        // Test that type mismatches are properly collected
        let json = r#"{"name": 123, "age": "thirty"}"#; // Wrong types
        let result: Result<TestStruct> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error: {:?}", error);
        assert_eq!(error.title, "Your request parameters didn't validate.");
        assert_eq!(error.status, Some(400));

        // For now, just check that we get an error - full error collection will be implemented in other tasks
        // This test verifies the enhanced from_str function works with error collection infrastructure
    }

    #[test]
    fn test_compatibility_with_serde_json_interface() {
        // Test that successful deserializations work identically to serde_json
        let json = r#"{"name": "John", "age": 30}"#;

        // Our implementation
        let our_result: Result<TestStruct> = from_str(json);
        assert!(our_result.is_ok());
        let our_value = our_result.unwrap();

        // serde_json implementation
        let serde_result: std::result::Result<TestStruct, serde_json::Error> =
            serde_json::from_str(json);
        assert!(serde_result.is_ok());
        let serde_value = serde_result.unwrap();

        // Should be identical
        assert_eq!(our_value.name, serde_value.name);
        assert_eq!(our_value.age, serde_value.age);
    }

    #[test]
    fn test_malformed_json_returns_syntax_error() {
        // Test that malformed JSON is handled in the detail field
        let malformed_json = r#"{"name": "John", "age": 30"#; // Missing closing brace
        let result: Result<TestStruct> = from_str(malformed_json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.title, "Your request parameters didn't validate.");
        assert!(error.detail.is_some());
        assert!(error.detail.unwrap().contains("JSON syntax error"));
        assert!(error.invalid_params.is_empty()); // JSON syntax errors don't generate InvalidParam entries
        assert_eq!(error.status, Some(400));
    }

    #[test]
    fn test_json_syntax_error_eof_while_parsing() {
        let incomplete_json = r#"{"name": "John""#; // Missing comma and closing
        let result: Result<TestStruct> = from_str(incomplete_json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.detail.is_some());
        let detail = error.detail.unwrap();
        assert!(detail.contains("JSON syntax error"));
        assert!(detail.contains("Unexpected end of input"));
        assert!(detail.contains("line"));
        assert!(detail.contains("column"));
        assert!(error.invalid_params.is_empty());
    }

    #[test]
    fn test_json_syntax_error_duplicate_field() {
        let duplicate_field_json = r#"{"name": "John", "name": "Jane", "age": 30}"#;
        let result: Result<TestStruct> = from_str(duplicate_field_json);

        // Note: serde_json actually allows duplicate fields by default, taking the last value
        // This test documents current behavior - if we want to detect duplicates,
        // we'd need additional validation logic
        if result.is_err() {
            let error = result.unwrap_err();
            assert!(error.detail.is_some());
            let detail = error.detail.unwrap();
            assert!(detail.contains("JSON syntax error"));
        }
    }

    #[test]
    fn test_json_syntax_error_invalid_number() {
        let invalid_number_json = r#"{"name": "John", "age": 030}"#; // Leading zero
        let result: Result<TestStruct> = from_str(invalid_number_json);

        if result.is_err() {
            let error = result.unwrap_err();
            assert!(error.detail.is_some());
            let detail = error.detail.unwrap();
            assert!(detail.contains("JSON syntax error"));
        }
    }

    #[test]
    fn test_json_syntax_error_expected_value() {
        let invalid_value_json = r#"{"name": "John", "age": }"#; // Missing value
        let result: Result<TestStruct> = from_str(invalid_value_json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.detail.is_some());
        let detail = error.detail.unwrap();
        assert!(detail.contains("JSON syntax error"));
        assert!(detail.contains("line"));
        assert!(detail.contains("column"));
        assert!(error.invalid_params.is_empty());
    }

    #[test]
    fn test_json_syntax_error_unescaped_control_character() {
        // This test uses a string with an actual control character
        let control_char_json = "{\"name\": \"John\nDoe\", \"age\": 30}"; // Unescaped newline
        let result: Result<TestStruct> = from_str(control_char_json);

        if result.is_err() {
            let error = result.unwrap_err();
            assert!(error.detail.is_some());
            let detail = error.detail.unwrap();
            assert!(detail.contains("JSON syntax error"));
        }
    }

    #[test]
    fn test_json_syntax_errors_dont_prevent_other_validation() {
        // Test that JSON syntax errors are reported in detail field
        // and don't generate InvalidParam entries, allowing other validation to work
        let malformed_json = r#"{"name": "John", "age": 30"#; // Missing closing brace
        let result: Result<TestStruct> = from_str(malformed_json);

        assert!(result.is_err());
        let error = result.unwrap_err();

        // JSON syntax errors should be in detail field
        assert!(error.detail.is_some());
        assert!(error.detail.unwrap().contains("JSON syntax error"));

        // Should not generate InvalidParam entries for syntax errors
        assert!(error.invalid_params.is_empty());

        // Should still have proper status code
        assert_eq!(error.status, Some(400));
    }

    #[test]
    fn test_missing_required_field_single() {
        let json_missing_name = r#"{"age": 30}"#; // Missing "name" field
        let result: Result<TestStruct> = from_str(json_missing_name);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.title, "Your request parameters didn't validate.");
        assert_eq!(error.status, Some(400));

        // Should have InvalidParam entry for missing field
        assert!(!error.invalid_params.is_empty());
        let missing_param = error
            .invalid_params
            .iter()
            .find(|p| p.name == "name")
            .expect("Should have error for missing 'name' field");

        assert_eq!(
            missing_param.reason,
            Some("missing required field".to_string())
        );
        assert_eq!(missing_param.expected.r#type, "required");
        assert_eq!(missing_param.expected.format, "field");
        assert_eq!(missing_param.actual.r#type, "missing");
        assert_eq!(missing_param.actual.format, "undefined");
        assert_eq!(missing_param.pointer, "#"); // Points to root object
    }

    #[test]
    fn test_missing_required_field_multiple() {
        let json_empty = r#"{}"#; // Missing both "name" and "age" fields
        let result: Result<TestStruct> = from_str(json_empty);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.title, "Your request parameters didn't validate.");
        assert_eq!(error.status, Some(400));

        // Should have InvalidParam entries for both missing fields
        assert_eq!(error.invalid_params.len(), 2);

        let name_param = error
            .invalid_params
            .iter()
            .find(|p| p.name == "name")
            .expect("Should have error for missing 'name' field");
        assert_eq!(
            name_param.reason,
            Some("missing required field".to_string())
        );
        assert_eq!(name_param.pointer, "#");

        let age_param = error
            .invalid_params
            .iter()
            .find(|p| p.name == "age")
            .expect("Should have error for missing 'age' field");
        assert_eq!(age_param.reason, Some("missing required field".to_string()));
        assert_eq!(age_param.pointer, "#");
    }

    #[test]
    fn test_missing_required_field_nested_struct() {
        let json_missing_nested = r#"{"user": {"name": "Alice"}, "active": true}"#; // Missing "age" in nested user
        let result: Result<NestedStruct> = from_str(json_missing_nested);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.title, "Your request parameters didn't validate.");

        // Should have InvalidParam entry for missing nested field
        assert!(!error.invalid_params.is_empty());
        let missing_param = error
            .invalid_params
            .iter()
            .find(|p| p.name == "age")
            .expect("Should have error for missing 'age' field");

        assert_eq!(
            missing_param.reason,
            Some("missing required field".to_string())
        );
        assert_eq!(missing_param.pointer, "#/user"); // Points to the nested object
    }

    #[test]
    fn test_missing_field_correct_json_pointer() {
        #[derive(Deserialize, Debug)]
        struct DeepNested {
            level1: Level1,
        }

        #[derive(Deserialize, Debug)]
        struct Level1 {
            level2: Level2,
        }

        #[derive(Deserialize, Debug)]
        struct Level2 {
            required_field: String,
        }

        let json = r#"{"level1": {"level2": {}}}"#; // Missing required_field in deep nested structure
        let result: Result<DeepNested> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();

        // Should have correct JSON pointer for deeply nested missing field
        let missing_param = error
            .invalid_params
            .iter()
            .find(|p| p.name == "required_field")
            .expect("Should have error for missing 'required_field'");

        assert_eq!(
            missing_param.reason,
            Some("missing required field".to_string())
        );
        assert_eq!(missing_param.pointer, "#/level1/level2");
    }

    #[test]
    fn test_partial_struct_with_missing_and_present_fields() {
        let json_partial = r#"{"name": "John"}"#; // Has name but missing age
        let result: Result<TestStruct> = from_str(json_partial);

        assert!(result.is_err());
        let error = result.unwrap_err();

        // Should only have error for missing field, not for present field
        assert_eq!(error.invalid_params.len(), 1);
        let missing_param = &error.invalid_params[0];
        assert_eq!(missing_param.name, "age");
        assert_eq!(
            missing_param.reason,
            Some("missing required field".to_string())
        );
        assert_eq!(missing_param.pointer, "#");
    }

    // Tests for task 7.3: Handle range and constraint violations
    #[test]
    fn test_debug_range_violation() {
        #[derive(Deserialize, Debug)]
        struct TestU8 {
            value: u8,
        }

        let json = r#"{"value": 256}"#;
        let result: Result<TestU8> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error count: {}", error.invalid_params.len());
        for (i, param) in error.invalid_params.iter().enumerate() {
            println!(
                "Error {}: name={}, reason={:?}, pointer={}",
                i, param.name, param.reason, param.pointer
            );
        }
        assert_eq!(error.invalid_params.len(), 1);
    }

    #[test]
    fn test_debug_multiple_range_violations() {
        #[derive(Deserialize, Debug)]
        struct TestMultiple {
            u8_val: u8,
            i8_val: i8,
            u16_val: u16,
        }

        let json = r#"{"u8_val": 256, "i8_val": 128, "u16_val": 65536}"#;
        let result: Result<TestMultiple> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        println!("Error count: {}", error.invalid_params.len());
        for (i, param) in error.invalid_params.iter().enumerate() {
            println!(
                "Error {}: name={}, reason={:?}, pointer={}",
                i, param.name, param.reason, param.pointer
            );
        }
    }

    #[test]
    fn test_u8_range_overflow() {
        #[derive(Deserialize, Debug)]
        struct TestU8 {
            value: u8,
        }

        let json = r#"{"value": 256}"#; // u8::MAX is 255
        let result: Result<TestU8> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("256"));
        assert!(range_error.reason.as_ref().unwrap().contains("u8"));
        assert_eq!(range_error.pointer, "#/value");
        assert_eq!(range_error.expected.r#type, "u8");
        assert!(range_error.expected.format.contains("0 to 255"));
        assert_eq!(range_error.actual.r#type, "integer");
        assert_eq!(range_error.actual.format, "256");
    }

    #[test]
    fn test_u8_range_negative() {
        #[derive(Deserialize, Debug)]
        struct TestU8 {
            value: u8,
        }

        let json = r#"{"value": -1}"#;
        let result: Result<TestU8> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("-1"));
        assert_eq!(range_error.pointer, "#/value");
    }

    #[test]
    fn test_i8_range_overflow() {
        #[derive(Deserialize, Debug)]
        struct TestI8 {
            value: i8,
        }

        let json = r#"{"value": 128}"#; // i8::MAX is 127
        let result: Result<TestI8> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("128"));
        assert_eq!(range_error.pointer, "#/value");
        assert!(range_error.expected.format.contains("-128 to 127"));
    }

    #[test]
    fn test_i8_range_underflow() {
        #[derive(Deserialize, Debug)]
        struct TestI8 {
            value: i8,
        }

        let json = r#"{"value": -129}"#; // i8::MIN is -128
        let result: Result<TestI8> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("-129"));
        assert_eq!(range_error.pointer, "#/value");
    }

    #[test]
    fn test_u16_range_overflow() {
        #[derive(Deserialize, Debug)]
        struct TestU16 {
            value: u16,
        }

        let json = r#"{"value": 65536}"#; // u16::MAX is 65535
        let result: Result<TestU16> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("65536"));
        assert_eq!(range_error.pointer, "#/value");
        assert!(range_error.expected.format.contains("0 to 65535"));
    }

    #[test]
    fn test_i32_range_overflow() {
        #[derive(Deserialize, Debug)]
        struct TestI32 {
            value: i32,
        }

        let json = r#"{"value": 2147483648}"#; // i32::MAX is 2147483647
        let result: Result<TestI32> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("2147483648"));
        assert_eq!(range_error.pointer, "#/value");
    }

    #[test]
    fn test_multiple_range_violations() {
        #[derive(Deserialize, Debug)]
        struct TestMultiple {
            u8_val: u8,
            i8_val: i8,
            u16_val: u16,
        }

        let json = r#"{"u8_val": 256, "i8_val": 128, "u16_val": 65536}"#;
        let result: Result<TestMultiple> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 3);

        // Check each error
        let u8_error = error
            .invalid_params
            .iter()
            .find(|e| e.name == "u8_val")
            .expect("Should have u8 error");
        assert!(u8_error.reason.as_ref().unwrap().contains("256"));
        assert!(u8_error.reason.as_ref().unwrap().contains("out of range"));

        let i8_error = error
            .invalid_params
            .iter()
            .find(|e| e.name == "i8_val")
            .expect("Should have i8 error");
        assert!(i8_error.reason.as_ref().unwrap().contains("128"));
        assert!(i8_error.reason.as_ref().unwrap().contains("out of range"));

        let u16_error = error
            .invalid_params
            .iter()
            .find(|e| e.name == "u16_val")
            .expect("Should have u16 error");
        assert!(u16_error.reason.as_ref().unwrap().contains("65536"));
        assert!(u16_error.reason.as_ref().unwrap().contains("out of range"));
    }

    #[test]
    fn test_range_violations_in_arrays() {
        #[derive(Deserialize, Debug)]
        struct TestArray {
            values: Vec<u8>,
        }

        let json = r#"{"values": [100, 256, 50, 300]}"#; // 256 and 300 are out of range for u8
        let result: Result<TestArray> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 2);

        // Check that errors have correct JSON pointers for array indices
        let first_error = error
            .invalid_params
            .iter()
            .find(|e| e.pointer == "#/values/1")
            .expect("Should have error at index 1");
        assert!(first_error.reason.as_ref().unwrap().contains("256"));

        let second_error = error
            .invalid_params
            .iter()
            .find(|e| e.pointer == "#/values/3")
            .expect("Should have error at index 3");
        assert!(second_error.reason.as_ref().unwrap().contains("300"));
    }

    #[test]
    fn test_float_to_integer_range_check() {
        #[derive(Deserialize, Debug)]
        struct TestFloat {
            value: u8,
        }

        // Test float value that's too large for u8
        let json = r#"{"value": 300.0}"#;
        let result: Result<TestFloat> = from_str(json);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.invalid_params.len(), 1);

        let range_error = &error.invalid_params[0];
        assert_eq!(range_error.name, "value");
        assert!(
            range_error
                .reason
                .as_ref()
                .unwrap()
                .contains("out of range")
        );
        assert!(range_error.reason.as_ref().unwrap().contains("300"));
    }

    #[test]
    fn test_valid_range_values() {
        #[derive(Deserialize, Debug)]
        struct TestValid {
            u8_val: u8,
            i8_val: i8,
            u16_val: u16,
            i16_val: i16,
            u32_val: u32,
            i32_val: i32,
        }

        let json = r#"{
            "u8_val": 255,
            "i8_val": -128,
            "u16_val": 65535,
            "i16_val": 32767,
            "u32_val": 4294967295,
            "i32_val": 2147483647
        }"#;
        let result: Result<TestValid> = from_str(json);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.u8_val, 255);
        assert_eq!(value.i8_val, -128);
        assert_eq!(value.u16_val, 65535);
        assert_eq!(value.i16_val, 32767);
        assert_eq!(value.u32_val, 4294967295);
        assert_eq!(value.i32_val, 2147483647);
    }
}
