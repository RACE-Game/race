use std::ops;

use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EntryLock {
    #[default]
    Open,
    JoinOnly,
    DepositOnly,
    Closed,
}

#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum BalanceChange {
    Add(u64),
    Sub(u64),
}

impl BalanceChange {
    fn amount(&self) -> i128 {
        match self {
            BalanceChange::Add(a) => *a as i128,
            BalanceChange::Sub(b) => -(*b as i128),
        }
    }
}

impl ops::Add<BalanceChange> for BalanceChange {
    type Output = BalanceChange;

    fn add(self, rhs: BalanceChange) -> BalanceChange {
        let mut sum: i128 = 0;
        sum += self.amount();
        sum += rhs.amount();
        if sum >= 0 {
            BalanceChange::Add(sum as u64)
        } else {
            BalanceChange::Sub(-sum as u64)
        }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Settle {
    pub player_id: u64,
    pub withdraw: u64,
    pub change: Option<BalanceChange>,
    pub eject: bool,
}

impl Settle {
    pub fn new(player_id: u64, withdraw: u64, change: Option<BalanceChange>, eject: bool) -> Self {
        Self { player_id, withdraw, change, eject }
    }
    pub fn is_empty(&self) -> bool {
        self.withdraw == 0 && self.change.is_none() && !self.eject
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Award {
    pub player_id: u64,
    pub bonus_identifier: String,
}

impl Award {
    pub fn new(player_id: u64, bonus_identifier: String) -> Self {
        Self { player_id, bonus_identifier }
    }
}

pub type Ciphertext = Vec<u8>;
pub type SecretDigest = Vec<u8>;
pub type SecretKey = Vec<u8>;


#[derive(Hash, Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SecretIdent {
    pub from_addr: String,
    pub to_addr: Option<String>,
    pub random_id: usize,
    pub index: usize,
}

impl SecretIdent {
    pub fn new_for_assigned<S: Into<String>>(
        random_id: usize,
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
        random_id: usize,
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
        random_id: usize,
        index: usize,
        secret: Vec<u8>,
    },
    Answer {
        from_addr: String,
        decision_id: usize,
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
        random_id: usize,
        index: usize,
        from_addr: String,
        to_addr: Option<String>,
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

    pub fn new_for_answer(decision_id: usize, from_addr: String, secret: SecretKey) -> Self {
        SecretShare::Answer {
            decision_id,
            from_addr,
            secret,
        }
    }
}


#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Transfer {
    pub amount: u64,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone)]
pub enum GameStatus {
    #[default]
    Idle,
    Running,
    Closed,
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatus::Idle => write!(f, "idle"),
            GameStatus::Running => write!(f, "running"),
            GameStatus::Closed => write!(f, "closed"),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GamePlayer {
    id: u64,
    position: u16,
}

impl GamePlayer {
    pub fn new(id: u64, position: u16) -> Self {
        Self {
            id,
            position,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn position(&self) -> u16 {
        self.position
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GameDeposit {
    id: u64,
    balance: u64,
    pub(crate) access_version: u64,
}

impl GameDeposit {
    pub fn new(id: u64, balance: u64, access_version: u64) -> Self {
        Self {
            id,
            balance,
            access_version,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlayerBalance {
    pub player_id: u64,
    pub balance: u64,
}

impl PlayerBalance {
    pub fn new(player_id: u64, balance: u64) -> Self {
        Self { player_id, balance }
    }
}
