pub trait Signer {
    fn sign(&self, raw: &[u8]) -> Vec<u8>;
}
