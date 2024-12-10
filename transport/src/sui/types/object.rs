//! Structs that represent Sui on-chain objects (those with UID or key capability)
mod game;
mod server;
mod profile;
mod recipient;
mod registry;

pub use game::*;
pub use server::*;
pub use profile::*;
pub use recipient::*;
pub use registry::*;
