pub type Ciphertext = Vec<u8>;
pub type SecretDigest = Vec<u8>;
pub type SecretKey = [u8; 44]; // key: 32, nonce: 12

pub fn empty_secret_key() -> SecretKey {
    [0u8; 44]
}
