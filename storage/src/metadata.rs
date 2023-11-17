//! Functionality for handling NFTs' metadata.json

use crate::constants::{MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH, RACE_LOGO_URI};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub uri: String, // max 200 chars
    #[serde(rename = "type")]
    pub mime: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Creator {
    pub address: String, // base64 string
    pub share: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Property {
    pub files: Vec<File>,
    pub category: String,
    pub creators: Vec<Creator>, // up to 5 creators
}

impl Property {
    fn add_file(&mut self, uri: String, mime: String) -> Result<()> {
        self.files.push(File { uri, mime });
        Ok(())
    }

    fn add_creator(&mut self, address: String) -> Result<()> {
        self.creators.push(Creator {
            address,
            share: 100,
        });
        Ok(())
    }
}

/// The struct is defined per Solana's NFT metadata.json spec.  See official doc:
/// https://docs.metaplex.com/programs/token-metadata/overview#nfts
/// Note: the official doc fails to emphasize the length limit of the fields `name`,
/// `symbol` and `creators`. See the above consts for reference.
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,   // max 32 chars
    pub symbol: String, // max 10 chars
    pub description: String,
    pub seller_fee_basis_points: u16,
    pub image: String, // url, max 200 chars
    pub external_url: String,
    pub attributes: Vec<String>,
    pub properties: Property,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: "Race Holdem".to_string(),
            symbol: "RACETEST".to_string(),
            description: "Race Game Bundle".to_string(),
            seller_fee_basis_points: 0,
            image: RACE_LOGO_URI.to_string(),
            external_url: "".to_string(),
            attributes: vec![],
            properties: Property {
                files: vec![],
                category: "image".to_string(),
                creators: vec![],
            },
        }
    }
}

impl Metadata {
    pub fn new(name: String, symbol: String, creator: String, bundle_addr: String) -> Result<Self> {
        if name.len() > MAX_NAME_LENGTH {
            return Err(Error::InvalidNameLength);
        } else if symbol.len() > MAX_SYMBOL_LENGTH {
            return Err(Error::InvalidSymbolLength);
        }

        Ok(Self {
            name,
            symbol,
            description: "Race Game Bundle".to_string(),
            seller_fee_basis_points: 0,
            image: RACE_LOGO_URI.to_string(),
            external_url: "".to_string(),
            attributes: vec![],
            properties: Property {
                files: vec![
                    File {
                        uri: RACE_LOGO_URI.to_string(),
                        mime: "image/png".to_string(),
                    },
                    File {
                        uri: bundle_addr,
                        mime: "application/wasm".to_string(),
                    },
                ],
                category: "image".to_string(),
                creators: vec![Creator {
                    address: creator,
                    share: 100,
                }],
            },
        })
    }

    pub fn add_file(&mut self, uri: String, mime: String) -> Result<()> {
        self.properties.add_file(uri, mime)?;
        Ok(())
    }

    pub fn add_creator(&mut self, addr: String) -> Result<()> {
        self.properties.add_creator(addr)?;
        Ok(())
    }
}
