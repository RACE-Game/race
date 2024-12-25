//! Types for on-chain objects and instructions (move calls)

mod params;
mod object;

// or only re-export specific items for public use
pub(crate) use params::*;
pub(crate) use object::*;
