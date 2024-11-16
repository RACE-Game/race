//! Decision handling
//!
//! Player can submit an immutable decision, and hide it from seeing by others
//! Later the decision can be revealed by share the secrets.

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum DecisionStatus {
    Asked,
    Answered,
    Releasing,
    Released,
}
