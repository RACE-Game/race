use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Credentials {
    pub ec_public: Vec<u8>,
    pub rsa_public: Vec<u8>,
    pub salt: Vec<u8>,
    pub ec_iv: Vec<u8>,
    pub rsa_iv: Vec<u8>,
    pub ec_private_enc: Vec<u8>,
    pub rsa_private_enc: Vec<u8>,
}
