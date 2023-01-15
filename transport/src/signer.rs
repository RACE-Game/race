pub trait Signer: Sync + Send {
    fn sign(&self, raw: &[u8]) -> Vec<u8>;
}
