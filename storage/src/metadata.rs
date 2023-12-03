use serde::Serialize;

use crate::{error::StorageError, solana::metadata::Metadata};

pub trait MetadataT: Serialize {
    fn json_vec(&self) -> Result<Vec<u8>, StorageError> {
        serde_json::to_vec(&self).map_err(|e| StorageError::SerializationError(e.to_string()))
    }
}

pub fn make_metadata(
    chain: &str,
    name: String,
    symbol: String,
    creator_addr: String,
    bundle_addr: String,
) -> Result<impl MetadataT, StorageError> {
    match chain {
        "solana" => Metadata::try_new(name, symbol, creator_addr, bundle_addr),
        _ => Err(StorageError::UnsupportedChain(chain.into())),
    }
}
