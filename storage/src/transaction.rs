//! Functionality for Arweave transactions using its HTTP API
use crate::{
    crypto,
    error::Result,
    merkle::{generate_leaves, generate_root, resolve_proofs, Node, Proof},
};
use infer;
use serde::{Deserialize, Serialize};

/// Recursive data structure required to calculate the signature field of a
/// transaction, according to arweave-js.  See:
/// https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/deepHash.ts
/// If DeepHashItem is the `List` variant, the `List` length (in raw bytes) and
/// the word "list" (in raw bytes) will be concated to get the initial acc.
/// This acc, along with `List` will be reduced until DeepHashItem becomes a `Blob`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeepHashItem {
    Blob(Vec<u8>),
    List(Vec<DeepHashItem>),
}

impl DeepHashItem {
    pub fn from_blob(item: &[u8]) -> DeepHashItem {
        Self::Blob(item.to_vec())
    }
    pub fn from_list(list: Vec<DeepHashItem>) -> DeepHashItem {
        Self::List(list)
    }
}

/// File or data tag per the Arweave spec.  When `value` is a hash256 string of data,
/// the string is treated as utf8 as well. More info:
/// docs.arweave.org/developers/arweave-node-server/http-api#transaction-format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    #[serde(with = "base64")]
    pub name: Vec<u8>,
    #[serde(with = "base64")]
    pub value: Vec<u8>,
}

impl Tag {
    fn new(name: Vec<u8>, value: Vec<u8>) -> Result<Tag> {
        Ok(Tag {name, value})
    }

    fn from_utf8_str(utf8_name: &str, utf8_value: &str) -> Result<Tag> {
        Ok(Tag {
            name: utf8_name.as_bytes().to_vec(),
            value: utf8_value.as_bytes().to_vec(),
        })
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = crypto::b64_encode(&self.name).unwrap();
        let value = crypto::b64_encode(&self.value).unwrap();
        write!(f, "name: {}, value: {}", name, value)
    }
}

// Reqeuest JSON struct per the Arweave spec.
// See: docs.arweave.org/developers/arweave-node-server/http-api#transaction-format
// This transaction currently supports only uploading data to Arweave.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transaction {
    // always 2
    pub format: u8,
    #[serde(with = "base64")]
    pub id: Vec<u8>,
    #[serde(with = "base64")]
    pub last_tx: Vec<u8>,
    #[serde(with = "base64")]
    pub owner: Vec<u8>,
    pub tags: Vec<Tag>,         // total size <= 2048 bytes
    #[serde(with = "base64")]
    pub target: Vec<u8>,        // "" for file/data transaction
    pub quantity: String,       // "0" for file/data transaction
    #[serde(with = "base64")]
    pub data_root: Vec<u8>,
    pub data_size: String,
    #[serde(with = "base64")]
    pub data: Vec<u8>,
    pub reward: String,         // winston amount represented in string
    #[serde(with = "base64")]
    pub signature: Vec<u8>,
    #[serde(skip)]
    pub chunks: Vec<Node>,
    #[serde(skip)]
    pub proofs: Vec<Proof>,
}

impl Transaction {
    pub fn new() -> Transaction {
        Transaction {
            format: 2,
            quantity: "0".to_string(),
            ..Default::default()
        }
    }

    fn merklize_data(&mut self, data: Vec<u8>) -> Result<Vec<u8>> {
        let mut chunks = generate_leaves(data)?;
        let root = generate_root(chunks.clone())?;
        let data_root = root.id.to_vec();
        let mut proofs = resolve_proofs(root, None)?;

        // Discard the last chunk & proof if it's zero length.
        let last_chunk = chunks.last().expect("should have at least one chunk");
        if last_chunk.max_byte_range == last_chunk.min_byte_range {
            chunks.pop();
            proofs.pop();
        }
        // set chunks and proofs
        self.chunks = chunks;
        self.proofs = proofs;

        Ok(data_root)
    }

    // Set data_root and data fields, also adding tags according to given data
    pub fn set_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.data_size = data.len().to_string();
        let data_root = self.merklize_data(data.clone())?;

        self.data_root = data_root;

        let data_hash = crypto::sha256_hash(&data)?;
        let file_hash = data_hash
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .concat();
        self.data = data;
        let content_type = if let Some(kind) = infer::get(&self.data) {
            kind.mime_type()
        } else {
            "application/octet-stream"
        };
        let content_tag = Tag::from_utf8_str("Content-Type", content_type)?;
        let hash_tag = Tag::new("File-Hash".as_bytes().to_vec(), crypto::b64_decode(&file_hash)?)?;
        self.tags.append(&mut vec![content_tag, hash_tag]);

        Ok(())
    }

    /// Compute deephash item in preparation for deep hashing.
    /// Supports only v2 format as v1 has been deprecated.  See:
    /// docs.arweave.org/developers/arweave-node-server/http-api#field-definitions
    fn get_deephash_item(&mut self) -> Result<DeepHashItem> {
        // This is the byte view of the tags for the data to be uploaded.
        // The iterator constructs [[name_in_bytes], [value_in_bytes]] and
        // the final structure looks like [[[name_in_bytes], [value_in_bytes]], ...]
        println!("== Going to deep hash the following fields:");
        println!("===========================================");
        println!("== Field `format`: {}", self.format.to_string());
        println!("== Field `owner`: {}", crypto::b64_encode(&self.owner)?);
        println!("== Field `target`: {}", crypto::b64_encode(&self.target)?);
        println!("== Field `quantity`: {}", self.quantity);
        println!("== Field `reward`: {}", self.reward);
        println!("== Field `last_tx`: {}", crypto::b64_encode(&self.last_tx)?);
        println!(
            "== Field `data_root`: {}",
            crypto::b64_encode(&self.data_root)?
        );
        println!("== Field `data_size`: {}", self.data_size);
        println!("== Field `tags` {:?}", self.tags);
        for tag in self.tags.iter() {
            println!("Tag: {}", tag);
        }
        println!("===========================================");

        let tag_list: Vec<DeepHashItem> = self
            .tags
            .iter()
            .map(|t| {
                DeepHashItem::from_list(vec![
                    DeepHashItem::from_blob(&t.name),
                    DeepHashItem::from_blob(&t.value),
                ])
            })
            .collect();

        let mut item_list: Vec<DeepHashItem> = vec![
            self.format.to_string().as_bytes(),
            &self.owner,
            &self.target,
            self.quantity.as_bytes(),
            self.reward.as_bytes(),
            &self.last_tx,
        ]
        .into_iter()
        .map(DeepHashItem::from_blob)
        .collect();
        item_list.push(DeepHashItem::from_list(tag_list));
        item_list.push(DeepHashItem::from_blob(self.data_size.as_bytes()));
        item_list.push(DeepHashItem::from_blob(&self.data_root));

        Ok(DeepHashItem::from_list(item_list))
    }

    /// Calculate the merkle root of a transaction's field as the input for its id
    /// docs.arweave.org/developers/arweave-node-server/http-api#transaction-signing
    pub fn get_deephash(&mut self) -> Result<[u8; 48]> {
        let deephash_item = self.get_deephash_item()?;
        // println!("== Deephash items {:?}", deephash_item);
        let deephash = crypto::deep_hash(deephash_item)?;
        Ok(deephash)
    }

    // Get id as base64url encoded string
    pub fn get_id(&self) -> Result<String> {
        let b64_id = crypto::b64_encode(&self.id)?;
        Ok(b64_id)
    }

    // Set the id field of the transaction
    pub fn set_id(&mut self, sig_hash: Vec<u8>) -> Result<()> {
        let id_hash = crypto::sha256_hash(&sig_hash)?;
        self.id = id_hash.to_vec();
        Ok(())
    }

    pub fn set_signature(&mut self, signature: &[u8]) -> Result<()> {
        self.signature = signature.to_vec();
        Ok(())
    }

    pub fn set_owner(&mut self, owner: &[u8]) -> Result<()> {
        self.owner = owner.to_vec();
        Ok(())
    }

    // Get reward as u64
    pub fn get_reward(&self) -> Result<u64> {
        let winston = self.reward.parse::<u64>()?;
        Ok(winston)
    }

    pub fn set_reward(&mut self, reward: String) -> Result<()> {
        self.reward = reward;
        Ok(())
    }

    pub fn set_last_tx(&mut self, base64url: &str) -> Result<()> {
        self.last_tx = crypto::b64_decode(base64url)?;
        Ok(())
    }

    pub fn unsigned(&self) -> bool {
        self.id.is_empty() || self.signature.is_empty()
    }
}

/// A custom de/serializing scheme is needed to handle all the base64 urls of a tx:
/// Serialize Vec<u8> to base64 url format upon sending the transaction.
/// Deserialize base64 url strings to Vec<u8> for convenience of computing hashes.
pub mod base64 {
    use serde::{Deserialize, Serialize};
    use serde::{Deserializer, Serializer, ser, de};
    use crate::crypto;
    // #[allow(clippy::ptr_arg)]
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let b64 = crypto::b64_encode(v).map_err(|_| ser::Error::custom("failed to encode raw bytes of base64 url string"))?;
        String::serialize(&b64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
            let base64url = String::deserialize(d)?;
            crypto::b64_decode(&base64url).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow;
    use std::fs;
    use std::path::PathBuf;


    #[test]
    fn test_encode_base64() -> anyhow::Result<()> {
        let b64 = vec![44; 7];
        assert_eq!(crypto::b64_encode(&b64)?, "LCwsLCwsLA");

        let raw_bytes = "Arweave".as_bytes();
        println!("-- Original: {}", String::from_utf8(raw_bytes.to_vec()).unwrap());
        let b64url = crypto::b64_encode(raw_bytes)?;
        assert_eq!(b64url, "QXJ3ZWF2ZQ");
        Ok(())
    }

    #[test]
    fn test_create_tag() -> anyhow::Result<()> {
        let tag = Tag::from_utf8_str("Content-Type", "application/wasm")?;
        let name_b64 = crypto::b64_encode(&tag.name)?;
        let value_b64 = crypto::b64_encode(&tag.value)?;
        println!("-- Tag: {}", tag);
        assert_eq!(name_b64, "Q29udGVudC1UeXBl".to_string());
        assert_eq!(value_b64, "YXBwbGljYXRpb24vd2FzbQ".to_string());
        Ok(())
    }

    #[test]
    fn test_serialize_tag() -> anyhow::Result<()> {
        let tag = Tag::from_utf8_str("Content-Type", "application/wasm")?;
        let tag_ser = serde_json::to_string(&tag)?;
        println!("Serialized tag {:?}", tag_ser);
        let tag_der: Tag = serde_json::from_str(&tag_ser)?;
        assert_eq!(tag.name, tag_der.name);
        Ok(())
    }

    #[test]
    fn test_set_data() -> anyhow::Result<()> {
        let file = "tests/holdem_cash.wasm";
        let raw = fs::read(PathBuf::from(file))?;

        let mut tx = Transaction::new();
        let data_root = tx.merklize_data(raw)?;
        let b64_data_root = crypto::b64_encode(&data_root)?;
        println!("-- Data root: {}", b64_data_root);

        assert_eq!(tx.chunks.len(), 2);
        assert_eq!(tx.proofs.len(), 2);
        assert_eq!(
            &b64_data_root,
            "3xEC-zrCwQVF88iSvOxbPHIZGIWySGiVKoe7n4rnidQ"
        );

        Ok(())
    }

    #[test]
    fn test_get_deephash() -> anyhow::Result<()> {
        let owner = "g1gL9QEVZ6yIXqom8ZFhkFfszVi2F9rZ1_oUFZQPSTAqu3QjECWxnkQgb9SQM7REFZJGX21LnZenPBaIeFay2S9_WYVvQEqjkxKPMnFE04i-q7qWetDyolzaElRdL8IvN4BG1nVePeWi1Z3-3aVjaat_p65LNdgaZ9heYyMnFq6XLfspLbfaa6_BNyzZjz6F-ME9ro8TDNgd3as-vmdhvTh3QNJqGWg6CGxkyBIPoCRVXw9ADvl-OAhgStpJJPVqo7wvp6teWTYu33JFyFadzkhU1s3oyIp4Np9tBYs6C96VwuT_0clUKSIb6f2CC__eClt3-aejmPrmTRS6Qhbhp3WhU5KRhvF7L-ya1AhgP_jmpnJTovhjjHQL9vY74lQfhN6M_SGvSchAJQd4bTkQf6x9tmEedKkZfK-ntA45uVD1LW3WPHYqIIeo2cBuaEbwK_csYgjVXNKym0guLgGNYVpAjSPLo7Eu1BFDbe0Gc8d0GOR4p7HaZf4X6udIP5ypF1bGlVDgCSSfYiSDAW5xv61_BPoXukVzoC7C6aP4OXz4p_9naUIce77SEbt19GOZg_9KZAUmtgZOxgsRm1nvyXiyBc2h87JF4KnSA1PJq4EMUsD3pt9vE2Uc9IZ9-7fOiycKYLFlXMVyhURjNCAYZA1sVVJXTWDP7mSoyEQAiqE";
        let last_tx = "KM7hEK5jmduDzSy4BxzbRrVgn2v5FCMAUfWYWF-da53xvgFUgTzzhvSnug9rV3yF";
        let droot = "3xEC-zrCwQVF88iSvOxbPHIZGIWySGiVKoe7n4rnidQ";
        let mut tx = Transaction {
            format: 2,
            last_tx: crypto::b64_decode(last_tx)?,
            owner: crypto::b64_decode(owner)?,
            tags: vec![
                Tag::from_utf8_str("Content-Type", "application/wasm")?,
                Tag::from_utf8_str(
                    "File-Hash",
                    // sha256 value treated as utf8_str in this case
                    "8a05c45f2fa6c04db66e8778b3a7e6b59bd94d9d94a1f533b5195225e33611ed"
                )?,
            ],
            quantity: 0.to_string(),
            data_root: crypto::b64_decode(droot)?,
            data_size: 482675.to_string(),
            reward: 421470902.to_string(),
            ..Transaction::default()
        };
        let deephash = tx.get_deephash()?;
        let arloader_deephash = [
            35, 162, 211, 83, 27, 221, 251, 145, 74, 158, 192, 17, 97, 97, 91, 173, 210, 111, 36,
            146, 45, 10, 137, 66, 181, 49, 170, 221, 191, 201, 72, 176, 213, 64, 56, 222, 63, 47,
            207, 92, 217, 157, 115, 110, 21, 179, 100, 59,
        ];
        assert_eq!(deephash, arloader_deephash);

        Ok(())
    }
}
