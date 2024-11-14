//! The data structures for on-chain accounts.

mod recipient_account;
mod game_account;
mod registration_account;
mod server_account;
mod token_account;
mod game_bundle;
mod player_profile;

pub use recipient_account::*;
pub use game_account::*;
pub use registration_account::*;
pub use server_account::*;
pub use token_account::*;
pub use game_bundle::*;
pub use player_profile::*;
