use serde::{Serialize, Deserialize};

#[allow(unused)]
pub type Addr = String;
#[allow(unused)]
pub type RandomId = usize;
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

#[derive(PartialEq, Eq)]
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
