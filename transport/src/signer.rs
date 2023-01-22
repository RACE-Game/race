pub trait Signer: Sync + Send {
    fn sign_raw(&self, raw: &[u8]) -> Vec<u8>;
}
