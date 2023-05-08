//! We used an enhanced 2-role based mental poker algorithmn among a few nodes.
//! Each node can either be a player or a transactor.
//! For each node, there are two modes for randomization:
//! 1. Shuffler: participate in the shuffling, hold the secrets
//! 2. Drawer: pick the random item by index

use std::collections::HashMap;
use std::sync::Mutex;

use aes::cipher::{KeyIvInit, StreamCipher};
use arrayref::{array_ref, array_refs, mut_array_refs};
use base64::Engine as _;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::{Signer, Verifier};
use openssl::{
    pkey::{Private, Public},
    rsa::{Padding, Rsa},
};
use race_core::encryptor::EncryptorT;
use race_core::types::Signature;
use rand::seq::SliceRandom;
use sha1::{Digest, Sha1};

use race_core::{
    encryptor::{EncryptorError, EncryptorResult},
    types::{Ciphertext, SecretDigest, SecretKey},
};

type Aes128Ctr64LE = ctr::Ctr64LE<aes::Aes128>;

fn base64_encode(data: &[u8]) -> String {
    println!("Length of data: {}", data.len());
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(data)
}

fn base64_decode(data: &str) -> EncryptorResult<Vec<u8>> {
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(data)
        .map_err(|_| EncryptorError::DecodeFailed)
}

#[derive(Debug)]
pub struct Encryptor {
    private_key: Rsa<Private>,
    default_public_key: Rsa<Public>,
    public_keys: Mutex<HashMap<String, Rsa<Public>>>,
}

impl Encryptor {
    pub fn try_new(private_key: Rsa<Private>) -> EncryptorResult<Self> {
        let public = Rsa::from_public_components(
            private_key
                .n()
                .to_owned()
                .or(Err(EncryptorError::ImportPrivateKeyError))?,
            private_key
                .e()
                .to_owned()
                .or(Err(EncryptorError::ImportPrivateKeyError))?,
        )
        .or(Err(EncryptorError::ImportPrivateKeyError))?;

        Ok(Self {
            private_key,
            default_public_key: public,
            public_keys: Mutex::new(HashMap::new()),
        })
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        let bits = 1024;
        let private = Rsa::generate(bits).expect("Failed to generate RSA keypair");
        Encryptor::try_new(private).expect("Failed to initiate encryptor")
    }
}

impl EncryptorT for Encryptor {
    fn gen_secret(&self) -> SecretKey {
        let mut secret = [0u8; 32];
        let (key, iv) = mut_array_refs![&mut secret, 16, 16];
        key.copy_from_slice(&rand::random::<[u8; 16]>());
        iv.copy_from_slice(&rand::random::<[u8; 16]>());
        secret.to_vec()
    }

    /// Encrypt the message use RSA public key
    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> EncryptorResult<Vec<u8>> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;

        let public = match addr {
            Some(addr) => public_keys
                .get(addr)
                .ok_or(EncryptorError::PublicKeyNotfound)?,
            None => &self.default_public_key,
        };
        let mut buf = vec![0; public.size() as _];
        let size = public
            .public_encrypt(text, &mut buf, Padding::PKCS1_OAEP)
            .map_err(|e| EncryptorError::RsaEncryptFailed(e.to_string()))?;
        Ok(buf[0..size].to_vec())
    }

    /// Decrypt the message use RSA private key
    fn decrypt(&self, text: &[u8]) -> EncryptorResult<Vec<u8>> {
        let mut buf = vec![0; self.private_key.size() as _];
        let size = self
            .private_key
            .private_decrypt(text, &mut buf, Padding::PKCS1_OAEP)
            .map_err(|e| EncryptorError::RsaDecryptFailed(e.to_string()))?;
        Ok(buf[0..size].to_vec())
    }

    fn sign_raw(&self, message: &[u8]) -> EncryptorResult<Vec<u8>> {
        let pkey = PKey::from_rsa(self.private_key.clone())
            .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)
            .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;
        signer
            .update(message)
            .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

        let signature = signer
            .sign_to_vec()
            .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

        Ok(signature)
    }

    fn sign(&self, message: &[u8], signer: String) -> EncryptorResult<Signature> {
        let timestamp = chrono::Utc::now().timestamp_millis() as _;
        let nonce: [u8; 8] = rand::random();
        let message = [message, &nonce, &u64::to_le_bytes(timestamp)].concat();
        let sig = self.sign_raw(&message)?;
        Ok(Signature {
            signer,
            nonce: base64_encode(&nonce),
            timestamp: timestamp as _,
            signature: base64_encode(&sig),
        })
    }

    fn verify_raw(
        &self,
        addr: Option<&str>,
        message: &[u8],
        signature: &[u8],
    ) -> EncryptorResult<()> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;
        let public = match addr {
            Some(addr) => public_keys
                .get(addr)
                .ok_or(EncryptorError::PublicKeyNotfound)?,
            None => &self.default_public_key,
        };

        let pkey = PKey::from_rsa(public.to_owned())
            .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
        let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey)
            .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
        verifier
            .update(message)
            .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
        verifier
            .verify(&signature)
            .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
        Ok(())
    }

    fn verify(&self, message: &[u8], signature: &Signature) -> EncryptorResult<()> {
        let Signature {
            signer,
            nonce,
            timestamp,
            signature,
        } = signature;
        // TODO: We should check timestamp here.
        let nonce = base64_decode(nonce)?;
        let signature = base64_decode(signature)?;
        let message = [message, &nonce, &u64::to_le_bytes(*timestamp)].concat();
        self.verify_raw(Some(&signer), &message, &signature)
    }

    fn apply(&self, secret: &SecretKey, buffer: &mut [u8]) {
        let secret = array_ref![secret, 0, 32];
        let (key, iv) = array_refs![secret, 16, 16];
        let mut cipher = Aes128Ctr64LE::new(key.into(), iv.into());
        cipher.apply_keystream(buffer);
    }

    fn apply_multi(&self, secrets: Vec<SecretKey>, buffer: &mut [u8]) {
        for secret in secrets.into_iter() {
            self.apply(secret.as_ref(), buffer);
        }
    }

    fn shuffle(&self, items: &mut Vec<Ciphertext>) {
        let mut rng = rand::thread_rng();
        items.shuffle(&mut rng);
    }

    fn add_public_key(&self, addr: String, raw: &str) -> EncryptorResult<()> {
        let mut public_keys = self
            .public_keys
            .lock()
            .map_err(|_| EncryptorError::AddPublicKeyError)?;

        let pubkey = Rsa::public_key_from_der(&base64_decode(raw)?)
            .map_err(|_| EncryptorError::ImportPublicKeyError)?;

        public_keys.insert(addr, pubkey);
        Ok(())
    }

    fn digest(&self, text: &[u8]) -> SecretDigest {
        Sha1::digest(text).to_vec()
    }

    fn export_public_key(&self, addr: Option<&str>) -> EncryptorResult<String> {
        let public_keys = self
            .public_keys
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;
        let pubkey = match addr {
            Some(addr) => public_keys
                .get(addr)
                .ok_or(EncryptorError::PublicKeyNotfound)?
                .public_key_to_der()
                .or(Err(EncryptorError::PublicKeyNotfound))?,
            None => self
                .private_key
                .public_key_to_der()
                .or(Err(EncryptorError::PublicKeyNotfound))?,
        };
        Ok(base64_encode(&pubkey))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use race_core::secret::SecretState;

    #[test]
    fn test_sign_verify() {
        let e = Encryptor::default();
        let text = b"hello";
        let sig = e.sign_raw(text).expect("Failed to sign");
        e.verify_raw(None, text, &sig).expect("Failed to verify");
    }

    #[test]
    fn test_encrypt_decrypt() {
        let e = Encryptor::default();
        let plain = e.gen_secret();
        let encrypted = e.encrypt(None, &plain).expect("Failed to encrypt");
        let decrypted = e.decrypt(&encrypted[..]).expect("Failed to decrypt");
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_export_public_key() -> anyhow::Result<()> {
        let privkey = "MIICdwIBADANBgkqhkiG9w0BAQEFAASCAmEwggJdAgEAAoGBAN2EJP9pElDhOrvuAnLrcCofkfZXmGqdi49O7OkG0nj0UWiqs+IbbToTINmrZt0Rcq+yVawo88sm18lsDCwSMn4fGZAR/qmYzznJzKetJnkW2OSwhvYcqcQVFkCWKOzDMh+ZNgxDSbTVPTQ9fC2X8EvXSKFuDzpaF7tg9I4gL/yBAgMBAAECgYBibtAJ9uS+r/brf43zBw/mh/TSZIZEChHz8nxv6CoquVZbjk800D8vKUTVtMaWwaQW0sYjJGeBBJeq16ppAwUQFm2v/H/yWHusinxum8t/pxL3GV8qNvbJoxdNeGJY8tKP5At8N+SL/tIJ9STf6mntLsd6lq2j6xpKuO7eMPUrgQJBAPgB24foIEdG9bVj3ee+r1H4IyNmSscv4x2nOMFxaWmMBBu/cdmEsI2fRTzIjXwUE21BQAI9yj+m8uwrmUsoOIkCQQDkp7oTalsufiXSMJ/JAjVy7S84S8OzR9ZduwL7NCbzXZ0g69mWgc4DGkdWghuAARL6Ql7+WwFwzDB0s1/VOrY5AkEAryBwqumpUWu0OeBJZEnsd09nUKn9B+ay08+vbjntm9B5Xjaz6EugeIENXTypXAK5LR80WeDUHlp/k3G+D6pZMQJBAMlyyCpQ2pKEize6pRvH+WUOeDql7X3m/YLIv2Cn2uUwhb26bJIAPItZPJ6HtEi7KYgYr25yqTtCejJm0jifKGkCQBDrmvF954+gn/Q5qXAMF8B7SxfbrwzIIsJNHkBKlmr3wIPwQst0sPrZwSOjDdnnEEuAGUEtLt7mxJpm5XpzLwI=";
        let pubkey = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDdhCT/aRJQ4Tq77gJy63AqH5H2V5hqnYuPTuzpBtJ49FFoqrPiG206EyDZq2bdEXKvslWsKPPLJtfJbAwsEjJ+HxmQEf6pmM85ycynrSZ5FtjksIb2HKnEFRZAlijswzIfmTYMQ0m01T00PXwtl/BL10ihbg86Whe7YPSOIC/8gQIDAQAB";
        let decoded = &base64_decode(privkey).unwrap();
        let privkey = PKey::private_key_from_pkcs8(decoded).unwrap();
        let e = Encryptor::try_new(privkey.rsa().unwrap()).unwrap();
        let exported_pubkey = e.export_public_key(None)?;
        assert_eq!(exported_pubkey, pubkey);
        Ok(())
    }

    #[test]
    fn test_apply() {
        let e = Encryptor::default();
        let text = b"hello";

        let secret1 = e.gen_secret();
        let secret2 = e.gen_secret();

        let mut buffer = text.clone();
        e.apply(&secret1, &mut buffer);
        e.apply(&secret2, &mut buffer);
        e.apply(&secret1, &mut buffer);
        e.apply(&secret2, &mut buffer);
        assert_eq!(&buffer, text);
    }

    #[test]
    fn test_mask_and_unmask() -> anyhow::Result<()> {
        let e = Arc::new(Encryptor::default());
        let mut state = SecretState::new(e);
        state.gen_random_secrets(1, 3);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let encrypted = state.mask(1, original_ciphertexts.clone())?;
        let decrypted = state.unmask(1, encrypted.clone())?;
        assert_ne!(original_ciphertexts, encrypted);
        assert_eq!(decrypted, original_ciphertexts);
        Ok(())
    }

    #[test]
    fn test_lock() -> anyhow::Result<()> {
        let e = Arc::new(Encryptor::default());
        let mut state = SecretState::new(e);
        state.gen_random_secrets(1, 3);
        let original_ciphertexts = vec![vec![41; 16], vec![42; 16], vec![43; 16]];
        let ciphertexts_and_tests = state.lock(1, original_ciphertexts)?;
        assert_eq!(3, ciphertexts_and_tests.len());
        Ok(())
    }
}
