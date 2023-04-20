pub mod error;
pub mod chain_type;
pub use chain_type::ChainType;

// Native-only
#[cfg(not(target_arch = "wasm32"))]
pub mod facade;
#[cfg(not(target_arch = "wasm32"))]
pub mod solana;
#[cfg(not(target_arch = "wasm32"))]
pub mod builder;
#[cfg(not(target_arch = "wasm32"))]
pub use builder::TransportBuilder;

// WASM-only
#[cfg(target_arch = "wasm32")]
pub mod facade_wasm;
#[cfg(target_arch = "wasm32")]
pub mod solana_wasm;
#[cfg(target_arch = "wasm32")]
pub mod wasm_trait;
#[cfg(target_arch = "wasm32")]
pub mod wasm_utils;
#[cfg(target_arch = "wasm32")]
pub mod wasm_builder;
#[cfg(target_arch = "wasm32")]
pub use wasm_builder::TransportBuilder;
#[cfg(target_arch = "wasm32")]
pub use wasm_trait::TransportLocalT;
