use gloo::console::error;
use race_transport::error::TransportError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum SdkError {
    #[error("JS invocation error: {0}")]
    InteropError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Connection(with transactor) error: {0}")]
    ConnectionError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<race_core::error::Error> for SdkError {
    fn from(value: race_core::error::Error) -> Self {
        Self::InternalError(value.to_string())
    }
}

impl From<SdkError> for TransportError {
    fn from(value: SdkError) -> Self {
        gloo::console::error!("An error occurred in transport:", value.to_string());
        TransportError::InteropError
    }
}

pub type Result<T> = std::result::Result<T, JsError>;
