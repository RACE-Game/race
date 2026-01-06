//! We used an enhanced 2-role based mental poker algorithmn among a few nodes.
//! Each node can either be a player or a transactor.
//! For each node, there are two modes for randomization:
//! 1. Shuffler: participate in the shuffling, hold the secrets
//! 2. Drawer: pick the random item by index

use std::collections::HashMap;
use std::sync::Mutex;

use arrayref::array_ref;
use base64::Engine as _;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;
use openssl::symm::{encrypt, Cipher};
use openssl::rand::rand_bytes;
use openssl::pkcs5::pbkdf2_hmac;
use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::ecdsa::EcdsaSig;
use openssl::hash::{hash, MessageDigest};
use openssl::nid::Nid;
use openssl::pkey::{HasPublic, PKey};
use openssl::{
    pkey::{Private, Public},
    rsa::{Padding, Rsa},
};
use race_core::credentials::Credentials;
use race_core::types::{SecretDigest, SecretKey};
use race_core::encryptor::{EncryptorError, EncryptorResult, EncryptorT, NodePublicKeyRaw};
use race_core::types::Signature;
use sha2::{Digest, Sha256};

// Since we use different secrets for each encryption,
// we can use a fixed IV.

#[allow(unused)]
fn aes_content_iv() -> Vec<u8> {
    vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

#[allow(unused)]
fn aes_digest_iv() -> Vec<u8> {
    vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
}

fn chacha20_iv() -> Vec<u8> {
    vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
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

fn chacha20_generate() -> EncryptorResult<Vec<u8>> {
    let secret = rand::random::<[u8; 32]>();
    Ok(secret.to_vec())
}

fn chacha20_encrypt(secret: &[u8], buffer: &mut [u8], iv: &[u8]) -> EncryptorResult<()> {
    let secret = array_ref![secret, 0, 32];
    let iv = array_ref![iv, 0, 12];
    let mut cipher = ChaCha20::new(secret.into(), iv.into());
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
    if signature.len() != 64 {
        return Err(EncryptorError::InvalidSignatureLength(signature.len()));
    }
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

#[allow(unused)]
fn import_rsa_private(raw: &str) -> EncryptorResult<Rsa<Private>> {
    let der = base64_decode(raw)?;
    let pkey =
        PKey::private_key_from_pkcs8(&der).map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    let key = pkey
        .rsa()
        .map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    Ok(key)
}

fn import_rsa_public(raw: &[u8]) -> EncryptorResult<Rsa<Public>> {
    let key = Rsa::public_key_from_der(&raw).map_err(|_| EncryptorError::ImportPublicKeyError)?;
    Ok(key)
}

#[allow(unused)]
fn import_rsa_public_str(raw: &str) -> EncryptorResult<Rsa<Public>> {
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

#[allow(unused)]
fn import_ec_private(raw: &str) -> EncryptorResult<EcKey<Private>> {
    let der = base64_decode(raw)?;
    let pkey =
        PKey::private_key_from_pkcs8(&der).map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    let key = pkey
        .ec_key()
        .map_err(|_| EncryptorError::ImportPrivateKeyError)?;
    Ok(key)
}

fn import_ec_public(raw: &[u8]) -> EncryptorResult<EcKey<Public>> {
    let key = EcKey::public_key_from_der(&raw).map_err(|_| EncryptorError::ImportPublicKeyError)?;
    Ok(key)
}

#[allow(unused)]
fn import_ec_public_str(raw: &str) -> EncryptorResult<EcKey<Public>> {
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

pub fn generate_credentials(
    original_secret: Vec<u8>,
) -> EncryptorResult<Credentials> {
    let rsa = rsa_generate()?;
    let ec = ec_generate()?;

    let mut salt = [0u8; 16];
    let mut rsa_iv = [0u8; 12];
    let mut ec_iv = [0u8; 12];

    rand_bytes(&mut salt).map_err(|_| EncryptorError::KeyGenFailed)?;
    rand_bytes(&mut rsa_iv).map_err(|_| EncryptorError::KeyGenFailed)?;
    rand_bytes(&mut ec_iv).map_err(|_| EncryptorError::KeyGenFailed)?;

    let mut key = [0u8; 32];
    let iterations = 100_000;

    pbkdf2_hmac(&original_secret, &salt, iterations, MessageDigest::sha256(), &mut key)
        .map_err(|_| EncryptorError::KeyGenFailed)?;

    let rsa_public = rsa.public_key_to_der().map_err(|_| EncryptorError::KeyGenFailed)?;
    let rsa_private_key_bytes = rsa.private_key_to_der().map_err(|_| EncryptorError::KeyGenFailed)?;

    let ec_public = ec.public_key_to_der().map_err(|_| EncryptorError::KeyGenFailed)?;
    let ec_private_key_bytes = ec.private_key_to_der().map_err(|_| EncryptorError::KeyGenFailed)?;

    let aes_gcm_cihper = Cipher::aes_256_gcm();
    let rsa_private_enc = encrypt(aes_gcm_cihper, &key, Some(&rsa_iv), &rsa_private_key_bytes).map_err(|_| EncryptorError::AesEncryptFailed)?;
    let ec_private_enc = encrypt(aes_gcm_cihper, &key, Some(&ec_iv), &ec_private_key_bytes).map_err(|_| EncryptorError::AesEncryptFailed)?;

    Ok(Credentials {
        ec_public,
        rsa_public,
        salt: salt.into(),
        ec_iv: ec_iv.into(),
        rsa_iv: rsa_iv.into(),
        ec_private_enc,
        rsa_private_enc,
    })
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
    publics: Mutex<HashMap<String, NodePublicKey>>,
}

impl Encryptor {
    pub fn try_new(private: NodePrivateKey) -> EncryptorResult<Self> {
        Ok(Self {
            private,
            publics: Mutex::new(HashMap::new()),
        })
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        let rsa = rsa_generate().expect("Failed to generate RSA keypair");
        let ec = ec_generate().expect("Failed to generate ECDSA keypair");
        Encryptor::try_new(NodePrivateKey { rsa, ec }).expect("Failed to initiate encryptor")
    }
}

impl EncryptorT for Encryptor {
    fn gen_secret(&self) -> SecretKey {
        chacha20_generate().expect("Failed to generate CHACHA20 key")
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
            signature,
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
        self.verify_raw(Some(signer), &message, signature)
    }

    fn apply(&self, secret: &SecretKey, buffer: &mut [u8]) {
        chacha20_encrypt(secret, buffer, &chacha20_iv()).unwrap();
    }

    fn apply_multi(&self, secrets: Vec<SecretKey>, buffer: &mut [u8]) {
        for secret in secrets.into_iter() {
            self.apply(secret.as_ref(), buffer);
        }
    }

    fn import_credentials(&self, addr: &str, credentials: Credentials) -> EncryptorResult<()> {
        let ec = import_ec_public(&credentials.ec_public)?;
        let rsa = import_rsa_public(&credentials.rsa_public)?;

        let mut public_keys = self
            .publics
            .lock()
            .map_err(|_| EncryptorError::AddPublicKeyError)?;

        public_keys.insert(addr.to_string(), NodePublicKey { rsa, ec });
        Ok(())
    }

    fn digest(&self, text: &[u8]) -> SecretDigest {
        Sha256::digest(text).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use race_core::secret::SecretState;

    // #[test]
    // fn test_decrypt_with_secrets() {
    //     let ciphertext_map = HashMap::from([(
    //         0,
    //         vec![
    //             76, 138, 120, 255, 162, 127, 170, 11, 107, 232, 184, 180, 152, 68, 232, 232, 63,
    //             145, 52, 43, 24,
    //         ],
    //     )]);
    //     let secret_map = HashMap::from([(
    //         0,
    //         vec![vec![
    //             12, 179, 151, 39, 145, 110, 76, 130, 36, 68, 73, 93, 67, 112, 241, 203,
    //         ]],
    //     )]);
    //     let encryptor = Encryptor::default();
    //     let decrypted = encryptor.decrypt_with_secrets(
    //         ciphertext_map,
    //         secret_map,
    //         &["OcPShKslbZKO5Gc_H-7WF".into()],
    //     );
    //     assert_eq!(
    //         decrypted,
    //         Ok(HashMap::from([(0, "OcPShKslbZKO5Gc_H-7WF".to_string())]))
    //     );
    // }

    #[test]
    fn st() -> anyhow::Result<()> {
        let mut ciphertext = vec![208, 106];
        let secrets =
            vec![22,109,18,116,185,237,253,196,132,144,46,195,182,168,247,198,222,131,31,215,50,85,47,55,112,162,217,128,134,239,84,100];

        let encryptor = Encryptor::default();
        encryptor.apply(&secrets, &mut ciphertext);

        println!("buf: {:?}", ciphertext);
        Ok(())
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
        let _decrypted = e.decrypt(&encrypted[..]).expect("Failed to decrypt");
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
    fn test_chacha20_encryption() -> anyhow::Result<()> {
        let aes = chacha20_generate()?;
        let mut message = vec![
            1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let message0 = message.clone();
        let iv = chacha20_iv();
        chacha20_encrypt(&aes, &mut message, &iv)?;
        chacha20_encrypt(&aes, &mut message, &iv)?;
        assert_eq!(message, message0);
        Ok(())
    }

    #[test]
    fn test_chacha20_decryption() -> anyhow::Result<()> {
        let key0 = vec![
            138, 2, 66, 90, 234, 68, 19, 246, 175, 29, 10, 13, 74, 4, 64, 21, 112, 68, 171, 44, 92,
            11, 216, 167, 131, 40, 225, 105, 201, 6, 0, 177,
        ];
        let key1 = vec![
            43, 255, 10, 33, 243, 30, 240, 125, 120, 183, 157, 103, 36, 210, 171, 15, 245, 215,
            115, 233, 144, 179, 251, 14, 113, 238, 162, 61, 57, 86, 202, 94,
        ];
        let mut data = vec![
            172, 220, 129, 28, 226, 219, 220, 47, 185, 176, 23, 69, 33, 157, 206, 82, 4, 212, 91,
            231, 115,
        ];
        let iv = chacha20_iv();
        chacha20_encrypt(&key0, &mut data, &iv)?;
        chacha20_encrypt(&key1, &mut data, &iv)?;
        let decrypted = String::from_utf8(data)?;
        assert_eq!(decrypted, "OcPShKslbZKO5Gc_H-7WF".to_string());
        Ok(())
    }

    #[test]
    fn test_rsa_creation() -> anyhow::Result<()> {
        let rsa = rsa_generate()?;
        let rsa_pub_raw = export_rsa_public(&rsa)?;
        let rsa_pub = import_rsa_public_string(&rsa_pub_raw)?;
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
        let chacha20 = chacha20_generate()?;
        let ciphertext = rsa_encrypt(&rsa, &chacha20)?;
        let decrypted = rsa_decrypt(&rsa, &ciphertext)?;
        assert_eq!(decrypted, chacha20);
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
        let result = ec_verify(&ec, &message, &signature)?;
        assert_eq!(result, true);
        Ok(())
    }

    #[test]
    fn test_ec_sign_2() -> anyhow::Result<()> {
        let ec_pub_raw = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAENM7uif/aovSW0n9+d6WLBZ3ofjyaqRjY7jO2pAbYELHZvtbtprmB2SqVkkiqaFMlm9nC+nOq/nXy2SIvWe02ug==";
        let _ec_priv_raw = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgWXPH+rxLqEJ6do3Pqlqug/maywivQbb57FtQadC/LtKhRANCAAQ0zu6J/9qi9JbSf353pYsFneh+PJqpGNjuM7akBtgQsdm+1u2muYHZKpWSSKpoUyWb2cL6c6r+dfLZIi9Z7Ta6";
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
