use wasm_bindgen::prelude::*;
pub struct Error(String);

impl From<race_core::error::Error> for Error {
    fn from(value: race_core::error::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<Error> for JsError {
    fn from(value: Error) -> Self {
        Self::new(&value.0)
    }
}

pub type Result<T> = std::result::Result<T, JsError>;
