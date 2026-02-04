#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};
use crate::types::ClientMode;

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum NodeStatus {
    Pending(u64),
    Confirming,
    Ready,
    Disconnected,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Pending(access_version) => write!(f, "pending[{}]", access_version),
            NodeStatus::Confirming => write!(f, "confirming"),
            NodeStatus::Ready => write!(f, "ready"),
            NodeStatus::Disconnected => write!(f, "disconnected"),
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Node {
    pub addr: String,
    pub id: u64,
    pub mode: ClientMode,
    pub status: NodeStatus,
}

impl Node {
    pub fn new_pending<S: Into<String>>(addr: S, access_version: u64, mode: ClientMode) -> Self {
        Self {
            addr: addr.into(),
            id: access_version,
            mode,
            status: NodeStatus::Pending(access_version),
        }
    }

    pub fn new<S: Into<String>>(addr: S, access_version: u64, mode: ClientMode) -> Self {
        Self {
            addr: addr.into(),
            id: access_version,
            mode,
            status: NodeStatus::Ready,
        }
    }
}
