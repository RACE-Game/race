use thiserror::Error;
use wasm_bindgen::prelude::*;

#[derive(Error)]
pub enum Error {
    #[error("Interop error: {0}")]
    InteropError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Connection(with transactor) error: {0}")]
    ConnectionError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<race_core::error::Error> for Error {
    fn from(value: race_core::error::Error) -> Self {
        Self::InternalError(value.to_string())
    }
}

impl From<Error> for JsError {
    fn from(value: Error) -> Self {
        Self::new(&value.0)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
