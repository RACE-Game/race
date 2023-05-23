#![allow(unused)]
//! We used an enhanced 2-role based mental poker algorithmn among a few nodes.
//! Each node can either be a player or a transactor.
//! For each node, there are two modes for randomization:
//! 1. Shuffler: participate in the shuffling, hold the secrets
//! 2. Drawer: pick the random item by index

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use aes::cipher::{KeyIvInit, StreamCipher};
use arrayref::{array_ref, array_refs, mut_array_refs};
use base64::Engine as _;
use openssl::aes::{aes_ige, AesKey};
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::hash::{hash, MessageDigest};
use openssl::nid::Nid;
use openssl::pkey::{HasPublic, PKey};
use openssl::sign::{Signer, Verifier};
use openssl::{
    pkey::{Private, Public},
    rsa::{Padding, Rsa},
};
use race_core::encryptor::{EncryptorT, NodePublicKeyRaw};
use race_core::types::Signature;
use rand::seq::SliceRandom;
use sha1::{Digest, Sha1};

use race_core::{
    encryptor::{EncryptorError, EncryptorResult},
    types::{Ciphertext, SecretDigest, SecretKey},
};

type Aes128Ctr64LE = ctr::Ctr64LE<aes::Aes128>;
type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;

// Since we use different secrets for each encryption,
// we can use a fixed IV.

fn aes_content_iv() -> Vec<u8> {
    vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

fn aes_digest_iv() -> Vec<u8> {
    vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
}

fn base64_encode(data: &[u8]) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    engine.encode(data)
}

fn base64_decode(data: &str) -> EncryptorResult<Vec<u8>> {
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(data)
        .map_err(|_| EncryptorError::DecodeFailed)
}

fn rsa_generate() -> EncryptorResult<Rsa<Private>> {
    let bits = 1024;
    Rsa::generate(bits).or(Err(EncryptorError::KeyGenFailed))
}

fn ec_generate() -> EncryptorResult<EcKey<Private>> {
    let curve =
        EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).or(Err(EncryptorError::KeyGenFailed))?;
    EcKey::generate(&curve).or(Err(EncryptorError::KeyGenFailed))
}

fn aes_generate() -> EncryptorResult<Vec<u8>> {
    let secret = rand::random::<[u8; 16]>();
    Ok(secret.to_vec())
}

fn aes_encrypt(secret: &[u8], buffer: &mut [u8], iv: &[u8]) -> EncryptorResult<()> {
    let mut cipher = Aes128Ctr64LE::new(secret.into(), iv.into());
    cipher.apply_keystream(buffer);
    Ok(())
}

fn rsa_encrypt<T>(key: &Rsa<T>, text: &[u8]) -> EncryptorResult<Vec<u8>>
where
    T: HasPublic,
{
    let mut buf = vec![0u8; key.size() as _];
    let size = key
        .public_encrypt(text, &mut buf, Padding::PKCS1_OAEP)
        .map_err(|e| EncryptorError::RsaEncryptFailed(e.to_string()))?;
    Ok(buf[0..size].to_vec())
}

fn rsa_decrypt(key: &Rsa<Private>, text: &[u8]) -> EncryptorResult<Vec<u8>> {
    let mut buf = vec![0; key.size() as _];
    let size = key
        .private_decrypt(text, &mut buf, Padding::PKCS1_OAEP)
        .map_err(|e| EncryptorError::RsaDecryptFailed(e.to_string()))?;
    Ok(buf[0..size].to_vec())
}

fn ec_sign(key: &EcKey<Private>, message: &[u8]) -> EncryptorResult<Vec<u8>> {
    let hashed = hash(MessageDigest::sha256(), message)
        .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

    let sig =
        EcdsaSig::sign(&hashed, key).map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

    let mut r = sig.r().to_vec();
    let s = sig.s().to_vec();

    r.extend(s);
    Ok(r)
}

fn ec_verify<T>(key: &EcKey<T>, message: &[u8], signature: &[u8]) -> EncryptorResult<bool>
where
    T: HasPublic,
{
    let sig_buf = array_ref![signature, 0, 64];
    let hashed = hash(MessageDigest::sha256(), message)
        .map_err(|e| EncryptorError::SignFailed(e.to_string()))?;

    let r = BigNum::from_slice(&sig_buf[0..32])
        .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
    let s = BigNum::from_slice(&sig_buf[32..64])
        .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
    let sig = EcdsaSig::from_private_components(r, s)
        .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
    let result = sig
        .verify(&hashed, key)
        .map_err(|e| EncryptorError::VerifyFailed(e.to_string()))?;
    Ok(result)
}

fn export_rsa_public<T>(key: &Rsa<T>) -> EncryptorResult<String>
where
    T: HasPublic,
{
    let der = key
        .public_key_to_der()
        .map_err(|_| EncryptorError::ExportPublicKeyError)?;
    Ok(base64_encode(&der))
}

fn import_rsa_private(raw: &str) -> EncryptorResult<Rsa<Private>> {
    let der = base64_decode(raw)?;
    let pkey =
        PKey::private_key_from_pkcs8(&der).map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    let key = pkey
        .rsa()
        .map_err(|e| EncryptorError::ImportPrivateKeyError)?;
    Ok(key)
}

fn import_rsa_public(raw: &str) -> EncryptorResult<Rsa<Public>> {
    let der = base64_decode(raw)?;
    let key = Rsa::public_key_from_der(&der).map_err(|_| EncryptorError::ImportPublicKeyError)?;
    Ok(key)
}

fn export_ec_public<T>(key: &EcKey<T>) -> EncryptorResult<String>
where
    T: HasPublic,
{
    let der = key
        .public_key_to_der()
        .map_err(|_| EncryptorError::ExportPublicKeyError)?;
    Ok(base64_encode(&der))
}

fn import_ec_private(raw: &str) -> EncryptorResult<EcKey<Private>> {
    let der = base64_decode(raw)?;
    let pkey =
        PKey::private_key_from_pkcs8(&der).map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    let key = pkey
        .ec_key()
        .map_err(|e| EncryptorError::ImportPrivateKeyError)?;
    Ok(key)
}

fn import_ec_public(raw: &str) -> EncryptorResult<EcKey<Public>> {
    let der = base64_decode(raw)?;
    let key = EcKey::public_key_from_der(&der).map_err(|_| EncryptorError::ImportPublicKeyError)?;
    Ok(key)
}

#[derive(Debug)]
pub struct NodePublicKey {
    rsa: Rsa<Public>,
    ec: EcKey<Public>,
}

impl TryInto<NodePublicKeyRaw> for &NodePublicKey {
    type Error = EncryptorError;

    fn try_into(self) -> Result<NodePublicKeyRaw, Self::Error> {
        Ok(NodePublicKeyRaw {
            rsa: export_rsa_public(&self.rsa)?,
            ec: export_ec_public(&self.ec)?,
        })
    }
}

#[derive(Debug)]
pub struct NodePrivateKey {
    rsa: Rsa<Private>,
    ec: EcKey<Private>,
}

impl TryInto<NodePublicKeyRaw> for &NodePrivateKey {
    type Error = EncryptorError;

    fn try_into(self) -> Result<NodePublicKeyRaw, Self::Error> {
        Ok(NodePublicKeyRaw {
            rsa: export_rsa_public(&self.rsa)?,
            ec: export_ec_public(&self.ec)?,
        })
    }
}

#[derive(Debug)]
pub struct Encryptor {
    private: NodePrivateKey,
    publics: Mutex<BTreeMap<String, NodePublicKey>>,
}

impl Encryptor {
    pub fn try_new(private: NodePrivateKey) -> EncryptorResult<Self> {
        Ok(Self {
            private,
            publics: Mutex::new(BTreeMap::new()),
        })
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        let bits = 1024;
        let decrypt_key = Rsa::generate(bits);
        let rsa = rsa_generate().expect("Failed to generate RSA keypair");
        let ec = ec_generate().expect("Failed to generate ECDSA keypair");
        Encryptor::try_new(NodePrivateKey { rsa, ec }).expect("Failed to initiate encryptor")
    }
}

impl EncryptorT for Encryptor {
    fn gen_secret(&self) -> SecretKey {
        aes_generate().expect("Failed to generate AES key")
    }

    /// Encrypt the message use RSA public key
    fn encrypt(&self, addr: Option<&str>, text: &[u8]) -> EncryptorResult<Vec<u8>> {
        let publics = self
            .publics
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;

        Ok(match addr {
            Some(addr) => rsa_encrypt(
                &publics
                    .get(addr)
                    .ok_or(EncryptorError::PublicKeyNotfound)?
                    .rsa,
                text,
            )?,
            None => rsa_encrypt(&self.private.rsa, text)?,
        })
    }

    /// Decrypt the message use RSA private key
    fn decrypt(&self, text: &[u8]) -> EncryptorResult<Vec<u8>> {
        rsa_decrypt(&self.private.rsa, text)
    }

    fn sign_raw(&self, message: &[u8]) -> EncryptorResult<Vec<u8>> {
        ec_sign(&self.private.ec, message)
    }

    fn sign(&self, message: &[u8], signer: String) -> EncryptorResult<Signature> {
        let timestamp = chrono::Utc::now().timestamp_millis() as _;
        let message = [message, &u64::to_le_bytes(timestamp)].concat();
        let signature = self.sign_raw(&message)?;
        Ok(Signature {
            signer,
            timestamp: timestamp as _,
            signature
        })
    }

    fn verify_raw(
        &self,
        addr: Option<&str>,
        message: &[u8],
        signature: &[u8],
    ) -> EncryptorResult<()> {
        let publics = self
            .publics
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;

        let res = match addr {
            Some(addr) => ec_verify(
                &publics
                    .get(addr)
                    .ok_or(EncryptorError::PublicKeyNotfound)?
                    .ec,
                message,
                signature,
            ),
            None => ec_verify(&self.private.ec, message, signature),
        }?;
        if res {
            Ok(())
        } else {
            Err(EncryptorError::VerifyFailed("Invalid signature".into()))
        }
    }

    fn verify(&self, message: &[u8], signature: &Signature) -> EncryptorResult<()> {
        let Signature {
            signer,
            timestamp,
            signature,
        } = signature;
        // TODO: We should check timestamp here.
        let message = [message, &u64::to_le_bytes(*timestamp)].concat();
        self.verify_raw(Some(&signer), &message, &signature)
    }

    fn apply(&self, secret: &SecretKey, buffer: &mut [u8]) {
        aes_encrypt(secret, buffer, &aes_content_iv());
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

    fn add_public_key(&self, addr: String, raw: &NodePublicKeyRaw) -> EncryptorResult<()> {
        let ec = import_ec_public(&raw.ec)?;
        let rsa = import_rsa_public(&raw.rsa)?;

        let mut public_keys = self
            .publics
            .lock()
            .map_err(|_| EncryptorError::AddPublicKeyError)?;

        public_keys.insert(addr, NodePublicKey { rsa, ec });
        Ok(())
    }

    fn digest(&self, text: &[u8]) -> SecretDigest {
        Sha1::digest(text).to_vec()
    }

    fn export_public_key(&self, addr: Option<&str>) -> EncryptorResult<NodePublicKeyRaw> {
        let publics = self
            .publics
            .lock()
            .map_err(|_| EncryptorError::ReadPublicKeyError)?;
        Ok(match addr {
            Some(addr) => publics
                .get(addr)
                .ok_or(EncryptorError::PublicKeyNotfound)?
                .try_into()?,
            None => (&self.private).try_into()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use race_core::secret::SecretState;

    #[test]
    fn test_decrypt_with_secrets() {
        let ciphertext_map = HashMap::from([(
            0,
            vec![
                76, 138, 120, 255, 162, 127, 170, 11, 107, 232, 184, 180, 152, 68, 232, 232, 63,
                145, 52, 43, 24,
            ],
        )]);
        let secret_map = HashMap::from([(
            0,
            vec![vec![
                12, 179, 151, 39, 145, 110, 76, 130, 36, 68, 73, 93, 67, 112, 241, 203,
            ]],
        )]);
        let encryptor = Encryptor::default();
        let decrypted = encryptor.decrypt_with_secrets(
            ciphertext_map,
            secret_map,
            &["OcPShKslbZKO5Gc_H-7WF".into()],
        );
        assert_eq!(
            decrypted,
            Ok(HashMap::from([(0, "OcPShKslbZKO5Gc_H-7WF".to_string())]))
        );
    }

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
    fn test_wrap_unwrap_secret() {
        let secret = vec![
            224, 94, 30, 52, 114, 149, 215, 66, 131, 241, 62, 207, 146, 217, 134, 53, 163, 3, 184,
            130, 159, 236, 218, 174, 92, 16, 212, 159, 91, 85, 103, 87,
        ];
        let e = Encryptor::default();
        let encrypted = e.encrypt(None, &secret).expect("Failed to encrypt");
        let decrypted = e.decrypt(&encrypted[..]).expect("Failed to decrypt");
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

    #[test]
    fn test_aes_encryption() -> anyhow::Result<()> {
        let aes = aes_generate()?;
        let mut message = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
        let message0 = message.clone();
        let iv = aes_content_iv();
        aes_encrypt(&aes, &mut message, &iv)?;
        aes_encrypt(&aes, &mut message, &iv)?;
        assert_eq!(message, message0);
        Ok(())
    }

    #[test]
    fn test_aes_encryption2() -> anyhow::Result<()> {
        let key0 = vec![
            12, 222, 155, 148, 235, 156, 43, 83, 5, 187, 70, 49, 58, 137, 103, 197,
        ];
        let key1 = vec![
            70, 133, 201, 204, 29, 186, 241, 49, 86, 56, 52, 254, 112, 190, 204, 45,
        ];
        let mut buf = vec![1, 2, 3, 4, 5, 6];
        let enc = vec![121, 191, 102, 191, 10, 55];
        let iv = aes_content_iv();
        aes_encrypt(&key0, &mut buf, &iv);
        aes_encrypt(&key1, &mut buf, &iv);
        assert_eq!(buf, enc);
        Ok(())
    }

    #[test]
    fn test_rsa_creation() -> anyhow::Result<()> {
        let rsa = rsa_generate()?;
        let rsa_pub_raw = export_rsa_public(&rsa)?;
        let rsa_pub = import_rsa_public(&rsa_pub_raw)?;
        let rsa_pub_raw0 = export_rsa_public(&rsa_pub)?;
        assert_eq!(rsa_pub_raw0, rsa_pub_raw);
        Ok(())
    }

    #[test]
    fn test_rsa_encryption() -> anyhow::Result<()> {
        let rsa = rsa_generate()?;
        let plaintext = vec![1u8, 2, 3, 4, 5, 6];
        let ciphertext = rsa_encrypt(&rsa, &plaintext)?;
        let decrypted = rsa_decrypt(&rsa, &ciphertext)?;
        assert_eq!(decrypted, plaintext);
        Ok(())
    }

    #[test]
    fn test_rsa_wrap_aes_key() -> anyhow::Result<()> {
        let rsa = rsa_generate()?;
        let aes = aes_generate()?;
        let ciphertext = rsa_encrypt(&rsa, &aes)?;
        let decrypted = rsa_decrypt(&rsa, &ciphertext)?;
        assert_eq!(decrypted, aes);
        Ok(())
    }

    #[test]
    fn test_rsa_wrap_aes_key2() -> anyhow::Result<()> {
        Ok(())
    }

    #[test]
    fn test_ec_creation() -> anyhow::Result<()> {
        let ec = ec_generate()?;
        let ec_pub_raw = export_ec_public(&ec)?;
        let ec_pub = import_ec_public(&ec_pub_raw)?;
        let ec_pub_raw0 = export_ec_public(&ec_pub)?;
        assert_eq!(ec_pub_raw0, ec_pub_raw);
        Ok(())
    }

    #[test]
    fn test_ec_sign() -> anyhow::Result<()> {
        let ec = ec_generate()?;
        let message = vec![1u8, 2, 3, 4, 5, 6];
        let signature = ec_sign(&ec, &message)?;
        println!("{:?}", signature);
        let result = ec_verify(&ec, &message, &signature)?;
        assert_eq!(result, true);
        Ok(())
    }

    #[test]
    fn test_ec_sign_2() -> anyhow::Result<()> {
        let ec_pub_raw = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAENM7uif/aovSW0n9+d6WLBZ3ofjyaqRjY7jO2pAbYELHZvtbtprmB2SqVkkiqaFMlm9nC+nOq/nXy2SIvWe02ug==";
        let ec_priv_raw = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWXPH+rxLqEJ6do3Pqlqug/maywivQbb57FtQadC/LtKhRANCAAQ0zu6J/9qi9JbSf353pYsFneh+PJqpGNjuM7akBtgQsdm+1u2muYHZKpWSSKpoUyWb2cL6c6r+dfLZIi9Z7Ta6";
        let sig = vec![
            52, 82, 48, 205, 188, 167, 214, 182, 12, 54, 55, 123, 207, 48, 215, 208, 185, 100, 34,
            120, 11, 185, 124, 101, 6, 24, 72, 251, 16, 237, 247, 97, 240, 29, 149, 67, 66, 171,
            94, 249, 48, 189, 46, 52, 100, 252, 94, 203, 85, 237, 200, 54, 248, 138, 25, 147, 15,
            180, 73, 111, 62, 87, 232, 54,
        ];
        let message = vec![1, 2, 3, 4, 5, 6];

        let ec_pub = import_ec_public(ec_pub_raw)?;
        let res = ec_verify(&ec_pub, &message, &sig)?;
        assert!(res);
        Ok(())
    }
}
