//! This crate contains the basics of the protocol
//! - Game handler interface
//! - Randomness implementation
//! - Encryption/decryption implementation

pub mod connection;
pub mod context;
pub mod encryptor;
pub mod engine;
pub mod error;
pub mod event;
pub mod random;
pub mod secret;
pub mod transport;
pub mod types;
