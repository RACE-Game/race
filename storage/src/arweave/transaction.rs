//! Functionality for Arweave transactions using its HTTP API.
//! This mod corresponds to the `lib/transaction.ts` module of arweave-js
use crate::arweave::{
    crypto,
    error::Result,
    merkle::{generate_leaves, generate_root, resolve_proofs, Node, Proof},
};
use infer;
use serde::{Deserialize, Serialize};
use tracing::debug;

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
    #[allow(dead_code)]
    fn new(name: Vec<u8>, value: Vec<u8>) -> Result<Tag> {
        Ok(Tag { name, value })
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

/// Chunk transaction used in posting data to [`/chunk` endpoint]( https://docs.arweave.org/developers/server/http-api#upload-chunks)
#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct Chunk {
    #[serde(with = "base64")]
    pub data_root: Vec<u8>,
    pub data_size: String,
    #[serde(with = "base64")]
    pub data_path: Vec<u8>,
    pub offset: String,
    #[serde(with = "base64")]
    pub chunk: Vec<u8>,
}

/// Reqeuest JSON struct per [Arweave spec on Chunk](https://docs.arweave.org/developers/arweave-node-server/http-api#transaction-format):
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
    pub tags: Vec<Tag>, // total size <= 2048 bytes
    #[serde(with = "base64")]
    pub target: Vec<u8>, // "" for file/data transaction
    pub quantity: String, // "0" for file/data transaction
    #[serde(with = "base64")]
    pub data: Vec<u8>,
    pub data_size: String,
    #[serde(with = "base64")]
    pub data_root: Vec<u8>,
    pub reward: String, // winston amount represented in string
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
    pub fn set_data(&mut self, data: Vec<u8>, content_type: Option<&str>) -> Result<()> {
        self.data_size = data.len().to_string();
        let data_root = self.merklize_data(data.clone())?;
        self.data_root = data_root;

        let data_hash = crypto::sha256_hash(&data)?;
        // Hexdecimal hash string treated as utf8 string in
        let file_hash = data_hash
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .concat();
        let content_type = if let Some(content_type) = content_type {
            content_type
        } else if let Some(kind) = infer::get(&data) {
            kind.mime_type()
        } else {
            "application/octet-stream"
        };
        let content_tag = Tag::from_utf8_str("Content-Type", content_type)?;
        let hash_tag = Tag::from_utf8_str("File-Hash", &file_hash)?;
        self.tags.append(&mut vec![content_tag, hash_tag]);

        self.data = data;

        Ok(())
    }

    fn log(&self) -> Result<()> {
        debug!("== Field `format`: {}", self.format.to_string());
        debug!("== Field `owner`: {}", crypto::b64_encode(&self.owner)?);
        debug!("== Field `target`: {}", crypto::b64_encode(&self.target)?);
        debug!("== Field `quantity`: {}", self.quantity);
        debug!("== Field `reward`: {}", self.reward);
        debug!("== Field `last_tx`: {}", crypto::b64_encode(&self.last_tx)?);
        debug!(
            "== Field `data_root`: {}",
            crypto::b64_encode(&self.data_root)?
        );
        debug!("== Field `data_size`: {}", self.data_size);
        debug!("== Field `tags` {:?}", self.tags);
        for tag in self.tags.iter() {
            debug!("Tag: {}", tag);
        }
        Ok(())
    }

    /// Compute deephash item in preparation for deep hashing.
    /// Supports only v2 format as v1 has been deprecated.  See:
    /// docs.arweave.org/developers/arweave-node-server/http-api#field-definitions
    fn get_deephash_item(&mut self) -> Result<DeepHashItem> {
        // This is the byte view of the tags for the data to be uploaded.
        // The iterator constructs [[name_in_bytes], [value_in_bytes]] and
        // the final structure looks like [[[name_in_bytes], [value_in_bytes]], ...]
        self.log()?;
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
        let deephash = crypto::deep_hash(deephash_item)?;
        Ok(deephash)
    }

    // Get a specific proof or offset at the given index
    fn get_data_path(&self, idx: usize) -> Vec<u8> {
        self.proofs
            .get(idx)
            .expect("proof should exist at the given index but found none")
            .proof()
            .clone()
    }

    fn get_offset(&self, idx: usize) -> usize {
        self.proofs
            .get(idx)
            .expect("offset should exist at the given index but found none")
            .offset()
    }

    // Get a specific slice of bytes form the data (or file to be uploaded)
    fn get_data_chunk(&self, idx: usize) -> Vec<u8> {
        let minbyte = self
            .chunks
            .get(idx)
            .expect("chunked data should exist at the given index but found none")
            .min_byte_range;
        let maxbyte = self
            .chunks
            .get(idx)
            .expect("chunked data should exist at the given index but found none")
            .max_byte_range;

        self.data[minbyte..maxbyte].to_vec()
    }

    /// Return a read-to-uoload [`Chunk`] from transaction for posting to `/chunk`
    pub fn get_chunk(&self, idx: usize) -> Result<Chunk> {
        Ok(Chunk {
            data_root: self.data_root.clone(),
            data_size: self.data_size.to_string(),
            data_path: self.get_data_path(idx),
            offset: self.get_offset(idx).to_string(),
            chunk: self.get_data_chunk(idx),
        })
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
    use crate::arweave::crypto;
    use serde::{de, ser, Deserializer, Serializer};
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let b64 = crypto::b64_encode(v)
            .map_err(|_| ser::Error::custom("failed to encode raw bytes of base64 url string"))?;
        String::serialize(&b64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64url = String::deserialize(d)?;
        crypto::b64_decode(&base64url).map_err(de::Error::custom)
    }
}
