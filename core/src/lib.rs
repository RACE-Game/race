//! This crate contains the basics of the protocol
//! - Game handler interface
//! - Randomness implementation
//! - Encryption/decryption implementation

#![feature(derive_default_enum)]

pub mod context;
pub mod engine;
pub mod error;
pub mod event;
pub mod random;
pub mod transport;
pub mod types;
