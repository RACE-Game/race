use crate::error::Result;
use crate::transaction::DeepHashItem;
use base64::{engine::general_purpose, Engine as _};
use openssl::{
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::{Padding, Rsa},
    sha,
    sign::{Signer, Verifier},
};

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

// Helper fns
pub fn b64_decode(b64str: &str) -> Result<Vec<u8>> {
    let engine = general_purpose::URL_SAFE_NO_PAD;
    let bytes = engine.decode(b64str)?;
    Ok(bytes)
}

pub fn b64_encode(bytes: &[u8]) -> Result<String> {
    let engine = general_purpose::URL_SAFE_NO_PAD;
    let b64str = engine.encode(bytes);
    Ok(b64str)
}

// BigNum representation of a JSON web key
#[derive(Debug)]
struct KeyComponents {
    n: Vec<u8>,
    e: Vec<u8>,
    d: Vec<u8>,
    p: Vec<u8>,
    q: Vec<u8>,
    dp: Vec<u8>,
    dq: Vec<u8>,
    qi: Vec<u8>,
}

impl KeyComponents {
    fn from_jwk(jwk: JsonWebKey) -> Result<Self> {
        let JsonWebKey {
            n,
            e,
            d,
            p,
            q,
            dp,
            dq,
            qi,
            ..
        } = jwk;

        Ok(Self {
            n: b64_decode(&n)?,
            e: b64_decode(&e)?,
            d: b64_decode(&d)?,
            p: b64_decode(&p)?,
            q: b64_decode(&q)?,
            dp: b64_decode(&dp)?,
            dq: b64_decode(&dq)?,
            qi: b64_decode(&qi)?,
        })
    }
}

/// JSON Web Key per the arweave spec.  More info:
/// https://docs.arweave.org/developers/arweave-node-server/http-api#key-format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonWebKey {
    #[allow(dead_code)]
    kty: String,
    #[allow(dead_code)]
    ext: bool,
    n: String, // modulus
    e: String, // exponent
    d: String,
    p: String,
    q: String,
    dp: String,
    dq: String,
    qi: String,
}

impl JsonWebKey {
    fn from_file(path: &str) -> Result<Self> {
        let file = PathBuf::from(path);
        let jwk_string = fs::read_to_string(file)?;
        let jwk: JsonWebKey = serde_json::from_str(&jwk_string)?;
        Ok(jwk)
    }

    fn to_rsa_keypair(jwk: JsonWebKey) -> Result<PKey<Private>> {
        let KeyComponents {
            n,
            e,
            d,
            p,
            q,
            dp,
            dq,
            qi,
        } = KeyComponents::from_jwk(jwk)?;

        let rsa = Rsa::from_private_components(
            BigNum::from_slice(&n)?,
            BigNum::from_slice(&e)?,
            BigNum::from_slice(&d)?,
            BigNum::from_slice(&p)?,
            BigNum::from_slice(&q)?,
            BigNum::from_slice(&dp)?,
            BigNum::from_slice(&dq)?,
            BigNum::from_slice(&qi)?,
        )?;
        let keypair = PKey::from_rsa(rsa)?;
        Ok(keypair)
    }
}

/// The key derived from user's local JsonWebKey for signing and verifying messages
#[derive(Debug)]
pub struct ArweaveKey {
    modulus: Vec<u8>,
    keypair: PKey<Private>,
}

impl ArweaveKey {
    pub fn wallet_addr(&self) -> Result<String> {
        let mut hasher = sha::Sha256::new();
        hasher.update(&self.modulus);
        let n_hash = hasher.finish();
        let addr = b64_encode(&n_hash)?;
        Ok(addr)
    }

    pub fn new_from_file(path: &str) -> Result<Self> {
        let jwk = JsonWebKey::from_file(path)?;
        let modulus = b64_decode(&jwk.n)?;
        let keypair = JsonWebKey::to_rsa_keypair(jwk)?;

        Ok(Self { modulus, keypair })
    }

    pub fn get_modulus(&self) -> Result<&[u8]> {
        // TODO: use Base64
        Ok(&self.modulus)
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair)?;
        // Padding is required
        signer.set_rsa_padding(Padding::PKCS1_PSS)?;
        signer.update(message)?;
        let signature = signer.sign_to_vec()?;
        Ok(signature)
    }

    pub fn verify(&self, message: &[u8], signature: &Vec<u8>) -> Result<bool> {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair)?;
        verifier.set_rsa_padding(Padding::PKCS1_PSS)?;
        verifier.update(message)?;
        Ok(verifier.verify(signature)?)
    }
}

pub fn sha256_hash(data: &[u8]) -> Result<[u8; 32]> {
    let hash = sha::sha256(data);
    Ok(hash)
}

// Hash a list of hashes and hash the concat-ed elements from the list
pub fn sha256_hash_all(messages: Vec<&[u8]>) -> Result<[u8; 32]> {
    let hash: Vec<[u8; 32]> = messages
        .into_iter()
        .map(|m| sha256_hash(m).expect("a hash256 value: [u8;32]"))
        .collect();
    let result = sha256_hash(&hash.concat())?;
    Ok(result)
}

fn sha384_hash(data: &[u8]) -> Result<[u8; 48]> {
    let hash = sha::sha384(data);
    Ok(hash)
}

fn sha384_hash_all(messages: Vec<&[u8]>) -> Result<[u8; 48]> {
    let hash: Vec<[u8; 48]> = messages
        .into_iter()
        .map(|m| sha384_hash(m).expect("a hash384 value: [u8;48]"))
        .collect();
    let result = sha384_hash(&hash.concat())?;
    Ok(result)
}

// Concat two sha384 values into a [u8;96] bytes array
fn concat_hash384(left: [u8; 48], right: [u8; 48]) -> Result<[u8; 96]> {
    let mut iter = left.into_iter().chain(right.into_iter());
    let result = [(); 96].map(|_| iter.next().unwrap());
    Ok(result)
}

/// Calculates signature of transaction in accordance with impl in arweave-js
/// https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/deepHash.ts
pub fn deep_hash(deep_hash_item: DeepHashItem) -> Result<[u8; 48]> {
    let hash = match deep_hash_item {
        DeepHashItem::Blob(blob) => {
            let blob_tag = format!("blob{}", blob.len());
            sha384_hash_all(vec![blob_tag.as_bytes(), &blob])?
        }
        DeepHashItem::List(list) => {
            let list_tag = format!("list{}", list.len());
            let mut hash = sha384_hash(list_tag.as_bytes())?;

            // Accumulate the hash until list reduced to a single blob
            for child in list.into_iter() {
                let child_hash = deep_hash(child)?;
                let hash_pair = concat_hash384(hash, child_hash)?;
                hash = sha384_hash(&hash_pair)?;
            }
            hash
        }
    };
    Ok(hash)
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow;

    #[test]
    fn test_read_jwk_from_file() -> anyhow::Result<()> {
        let file_path = "/home/noel/projects/wallets/arweave.json";

        let jwk = JsonWebKey::from_file(file_path).unwrap();
        println!("-- Json Web Key {:?}", jwk);
        assert_eq!(jwk.kty, "RSA".to_string());
        assert_eq!(jwk.e, "AQAB".to_string());
        assert!(jwk.ext);
        Ok(())
    }

    #[test]
    fn test_modulus_len() -> anyhow::Result<()> {
        let file_path = "/home/noel/projects/wallets/arweave.json";
        let arkey = ArweaveKey::new_from_file(file_path).unwrap();
        assert_eq!(arkey.keypair.bits(), 4096);
        Ok(())
    }

    #[test]
    fn test_wallet_addr() -> anyhow::Result<()> {
        let file_path = "/home/noel/projects/wallets/arweave.json";
        let arkey = ArweaveKey::new_from_file(file_path).unwrap();
        let addr = arkey.wallet_addr()?;
        println!("wallet address: {}", addr);
        assert_eq!(
            addr,
            "2kYPVG9RHmU8bIKecI-2U2JZojwfsyLQ-rHayJhf78E".to_string()
        );
        Ok(())
    }

    #[test]
    fn test_sign_verify() -> anyhow::Result<()> {
        let file_path = "/home/noel/projects/wallets/arweave.json";
        let arkey = ArweaveKey::new_from_file(file_path).unwrap();
        let message = b"arweave";
        let signature = arkey.sign(message)?;
        println!("Signature leng {}", signature.len());
        println!("Signature hash {:?}", signature);
        assert!(arkey.verify(message, &signature)?);

        Ok(())
    }
}
