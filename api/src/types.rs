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

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Settle {
    pub player_id: u64,
    pub amount: u64,
    pub eject: bool,
}

impl Settle {
    pub fn new(player_id: u64, amount: u64, eject: bool) -> Self {
        Self { player_id, amount, eject }
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
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

pub type Addr = String;
pub type Amount = u64;
pub type RandomId = usize;
pub type DecisionId = usize;
pub type Ciphertext = Vec<u8>;
pub type SecretDigest = Vec<u8>;
pub type SecretKey = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
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


#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Transfer {
    pub slot_id: u8,
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
