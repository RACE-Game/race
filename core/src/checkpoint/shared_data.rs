use crate::node::Node;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};
use race_api::types::PlayerBalance;

/// The general information for a game.
/// This information is shared among master game and sub games.
#[derive(Default, Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SharedData {
    /// The balance of each players.  This information is available
    /// on-chain, but it's necessary if we want to resume a game from
    /// only checkpoint.
    pub balances: Vec<PlayerBalance>,
    /// The relationship between the addresses and game IDs.
    pub nodes: Vec<Node>,
}

impl SharedData {
    pub fn new(balances: Vec<PlayerBalance>, nodes: Vec<Node>) -> Self {
        Self { balances, nodes }
    }
}
