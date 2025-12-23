use race_api::event::Event;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DispatchEvent {
    pub timeout: u64,
    pub event: Event,
}

impl DispatchEvent {
    pub fn new(event: Event, timeout: u64) -> Self {
        Self { timeout, event }
    }
}
