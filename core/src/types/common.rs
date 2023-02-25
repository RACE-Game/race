use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[allow(unused)]
pub type Addr = String;
#[allow(unused)]
pub type Amount = u64;
#[allow(unused)]
pub type RandomId = usize;
pub type RandomIndex = usize;
pub type DecisionId = usize;
pub type Ciphertext = Vec<u8>;
pub type SecretDigest = Vec<u8>;
pub type SecretKeyRaw = [u8; 44]; // key: 32, nonce: 12
                                  // There's an issue for serialization of arrary,
                                  // So we have this vector type.
pub type SecretKey = Vec<u8>;

pub fn empty_secret_key_raw() -> SecretKeyRaw {
    [0u8; 44]
}

pub fn empty_secret_key() -> SecretKey {
    vec![0u8; 44]
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClientMode {
    Player,
    Transactor,
    Validator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub signer: String,
    pub nonce: String,
    pub timestamp: u64,
    pub signature: String,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}](signer: {}, timestamp: {}, nonce: {})",
            self.signature, self.signer, self.timestamp, self.nonce
        )
    }
}

#[derive(
    Hash,
    Debug,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
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

#[derive(
    Hash, Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq, Eq,
)]
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
        index: RandomIndex,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum VoteType {
    ServerVoteTransactorDropOff,
    ClientVoteTransactorDropOff,
}
