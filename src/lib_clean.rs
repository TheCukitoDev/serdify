use serde::{Deserialize, Serialize};
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

impl<T> Result<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, Result::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Result::Err(_))
    }

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
    match serde_json::from_str(&s) {
        Ok(t) => Result::Ok(t),
        Err(_e) => Result::Err(Error {
            r#type: None,
            title: "Deserialization failed".to_string(),
            detail: Some("JSON parsing error".to_string()),
            instance: None,
            invalid_params: vec![],
            status: Some(400),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
