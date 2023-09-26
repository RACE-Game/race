use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum SettleOp {
    Add(u64),
    Sub(u64),
    Eject,
    AssignSlot(String),
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Settle {
    pub addr: String,
    pub op: SettleOp,
}

impl Settle {
    pub fn add<S: Into<String>>(addr: S, amount: u64) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Add(amount),
        }
    }
    pub fn sub<S: Into<String>>(addr: S, amount: u64) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Sub(amount),
        }
    }
    pub fn eject<S: Into<String>>(addr: S) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Eject,
        }
    }
    pub fn assign<S: Into<String>>(addr: S, identifier: String) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::AssignSlot(identifier),
        }
    }
}

pub type Addr = String;
pub type Amount = u64;
pub type RandomId = usize;
pub type DecisionId = usize;
pub type Ciphertext = Vec<u8>;
pub type SecretDigest = Vec<u8>;
pub type SecretKey = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature {
    pub signer: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}](signer: {}, timestamp: {})",
            self.signature, self.signer, self.timestamp
        )
    }
}

#[derive(Hash, Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SecretIdent {
    pub from_addr: String,
    pub to_addr: Option<String>,
    pub random_id: RandomId,
    pub index: usize,
}

impl SecretIdent {
    pub fn new_for_assigned<S: Into<String>>(
        random_id: RandomId,
        index: usize,
        from_addr: S,
        to_addr: S,
    ) -> Self {
        SecretIdent {
            from_addr: from_addr.into(),
            to_addr: Some(to_addr.into()),
            random_id,
            index,
        }
    }

    pub fn new_for_revealed<S: Into<String>>(
        random_id: RandomId,
        index: usize,
        from_addr: S,
    ) -> Self {
        SecretIdent {
            from_addr: from_addr.into(),
            to_addr: None,
            random_id,
            index,
        }
    }
}

#[derive(Hash, Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum SecretShare {
    Random {
        from_addr: String,
        to_addr: Option<String>,
        random_id: RandomId,
        index: usize,
        secret: Vec<u8>,
    },
    Answer {
        from_addr: String,
        decision_id: DecisionId,
        secret: Vec<u8>,
    },
}

impl std::fmt::Display for SecretShare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretShare::Random {
                from_addr,
                to_addr,
                random_id,
                index,
                ..
            } => {
                write!(
                    f,
                    "#{}[{}]=>[{}]@{}",
                    random_id,
                    from_addr,
                    match to_addr {
                        Some(ref addr) => addr.as_str(),
                        None => "ALL",
                    },
                    index
                )
            }
            SecretShare::Answer {
                from_addr,
                decision_id,
                ..
            } => {
                write!(f, "#{}[{}]", decision_id, from_addr)
            }
        }
    }
}

impl SecretShare {
    pub fn new_for_random(
        random_id: RandomId,
        index: usize,
        from_addr: Addr,
        to_addr: Option<Addr>,
        secret: SecretKey,
    ) -> Self {
        SecretShare::Random {
            from_addr,
            to_addr,
            random_id,
            index,
            secret,
        }
    }

    pub fn new_for_answer(decision_id: DecisionId, from_addr: Addr, secret: SecretKey) -> Self {
        SecretShare::Answer {
            decision_id,
            from_addr,
            secret,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum VoteType {
    ServerVoteTransactorDropOff,
    ClientVoteTransactorDropOff,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum RecipientSlotType {
    Nft,
    Token,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum RecipientSlotOwner {
    Unassigned { identifier: String },
    Assigned { addr: String },
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RecipientSlotShare {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
    pub claim_amount: u64,
    pub claim_amount_cap: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RecipientSlot {
    pub id: u8,
    pub slot_type: RecipientSlotType,
    pub token_addr: String,
    pub shares: Vec<RecipientSlotShare>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EntryType {
    /// A player can join the game by sending assets to game account directly
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Cash { min_deposit: u64, max_deposit: u64 },
    /// A player can join the game by pay a ticket.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Ticket {
        slot_id: u8,
        amount: u64,
    },
    /// A player can join the game by showing a gate NFT
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Gating { collection: String },
}

impl Default for EntryType {
    fn default() -> Self {
        EntryType::Cash {
            min_deposit: 0,
            max_deposit: 1000000,
        }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Transfer {
    pub slot_id: u8,
    pub amount: u64,
}

/// Represent a player call the join instruction in contract.
#[derive(Debug, Default, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlayerJoin {
    pub addr: String,
    pub position: u16,
    pub balance: u64,
    pub access_version: u64,
    pub verify_key: String,
}

impl PlayerJoin {
    pub fn new<S: Into<String>>(
        addr: S,
        position: u16,
        balance: u64,
        access_version: u64,
        verify_key: String,
    ) -> Self {
        Self {
            addr: addr.into(),
            position,
            balance,
            access_version,
            verify_key,
        }
    }
}

/// Represent a player call the deposit instruction in contract.
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlayerDeposit {
    pub addr: String,
    pub amount: u64,
    pub settle_version: u64,
}

impl PlayerDeposit {
    pub fn new<S: Into<String>>(addr: S, balance: u64, settle_version: u64) -> Self {
        Self {
            addr: addr.into(),
            amount: balance,
            settle_version,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ServerJoin {
    pub addr: String,
    pub endpoint: String,
    pub access_version: u64,
    pub verify_key: String,
}

impl ServerJoin {
    pub fn new<S: Into<String>>(
        addr: S,
        endpoint: String,
        access_version: u64,
        verify_key: String,
    ) -> Self {
        Self {
            addr: addr.into(),
            endpoint,
            access_version,
            verify_key,
        }
    }
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone)]
pub enum GameStatus {
    #[default]
    Uninit,
    Running,
    Closed,
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatus::Uninit => write!(f, "uninit"),
            GameStatus::Running => write!(f, "running"),
            GameStatus::Closed => write!(f, "closed"),
        }
    }
}
