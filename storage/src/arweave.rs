//! The API module of this crate which is used to construct the needed json file
//! for uploading files to Awearve using its HTTP API
//! This mod corresponds to the `common/transactions.ts` module of arweave-js

pub mod b64;
pub mod constants;
pub mod crypto;
pub mod error;
pub mod merkle;
pub mod transaction;

use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
    Client, Response,
};

use crate::arweave::{
    constants::*,
    crypto::ArweaveKey,
    error::{Error, Result},
    transaction::{Chunk, Transaction},
};
use serde_json;
use tracing::{error, info};

/// Struct with methods for interacting with the Arweave network.
pub struct Arweave {
    pub base_url: String,
    pub arkey: ArweaveKey,
}

impl Arweave {
    pub fn try_new(keypath: &str) -> Result<Self> {
        Ok(Self {
            base_url: String::from("https://arweave.net/"),
            arkey: ArweaveKey::new_from_file(keypath)?,
        })
    }

    fn make_endpoint(&self, endpoint: &str) -> String {
        let mut url = self.base_url.clone();
        url.push_str(endpoint);
        url
    }

    // Fetch the needed Winston for given data size
    // 1 AR = 1 * 10^12 Winston
    pub async fn get_price(&self, data_size: usize) -> Result<String> {
        let query = format!("price/{}", &data_size.to_string());
        let url = self.make_endpoint(&query);
        let price = reqwest::get(&url)
            .await
            .map_err(|e| Error::ArweaveGetPriceError(e))?
            .text()
            .await?;
        Ok(price)
    }

    pub async fn get_last_tx(&self) -> Result<String> {
        let url = self.make_endpoint(AR_ANCHOR);
        let last_tx = reqwest::get(&url)
            .await
            .map_err(|e| Error::ArweaveLastTxError(e))?
            .text()
            .await?;
        Ok(last_tx)
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let addr = self.arkey.wallet_addr()?;
        let query = format!("wallet/{}/balance", addr);
        let url = self.make_endpoint(&query);
        let balance = reqwest::get(&url)
            .await
            .map_err(|e| Error::ArweaveWalletBalanceError(e))?
            .text()
            .await?
            .parse::<u64>()?;
        Ok(balance)
    }

    /// Build a transaction with the given data and set up the following fields
    /// - format, quantity, target
    /// - reward, last_tx
    /// - data, data_size, data_root, tags
    ///
    /// After the creation, fields that remain empty are id, signature and owner.
    /// They will be set when calling [`sign_transaction`] on [`arweave::Arweave`].
    pub async fn create_transaction(&self, data: Vec<u8>, content_type: Option<&str>) -> Result<Transaction> {
        let mut tx = Transaction::new();

        let reward = self.get_price(data.len()).await?;
        let last_tx = self.get_last_tx().await?;
        tx.set_reward(reward)?;
        tx.set_last_tx(&last_tx)?;

        tx.set_data(data, content_type)?;

        Ok(tx)
    }

    pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<()> {
        tx.set_owner(&self.arkey.get_modulus()?)?;
        let deephash = tx.get_deephash()?;
        let signature = self.arkey.sign(&deephash)?;
        info!("Signature length {}", signature.len());
        info!(
            "Signature base64url {:?}",
            crypto::b64_encode(&signature).unwrap()
        );
        tx.set_signature(&signature)?;
        tx.set_id(signature)?;
        Ok(())
    }

    /// Send post request to Arweave `chunk/` or endpoint for uploading data chunks.
    /// The actual uploading progress is explained in the doc of `uploadChunk`:
    /// https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/transaction-uploader.ts#L85-L88
    pub async fn post_chunk(&mut self, chunk: Chunk) -> Result<Response> {
        let client = Client::new();
        let url = self.make_endpoint("chunk");
        info!("Post chunks to {}", url);
        let json_chunk = serde_json::to_string(&chunk)?;
        let resp = client
            .post(&url)
            .header(&ACCEPT, "application/json")
            .header(&CONTENT_TYPE, "application/json")
            .body(json_chunk.clone())
            .send()
            .await?;
        Ok(resp)
    }

    pub async fn post_transaction(&mut self, signed_tx: Transaction) -> Result<(String, u64)> {
        if signed_tx.unsigned() {
            return Err(Error::UnsignedTransaction.into());
        }

        // Serialize the transaction to string json format.  The `.json()` method
        // form reqwest::RequestBuilder serializes transaction to Vec<u8> by default.
        // this requires all raw bytes of the transaction to be converted into their
        // base64 url representations beforehand.  Sending the json-stringified body
        // avoids the redundent conversion.

        let mut ntx = signed_tx.clone();
        ntx.data = vec![];
        let json_tx = serde_json::to_string(&ntx)?;

        let url = self.make_endpoint("tx");

        let client = Client::new();
        let resp = client
            .post(&url)
            .header(&CONTENT_TYPE, "application/json")
            .header(&ACCEPT, "application/json text/plain, */*")
            .body(json_tx.clone())
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if status != reqwest::StatusCode::OK {
            error!("Arweave upload failed, due to {}", body);
            return Err(Error::StatusCodeNotOk)
        }

        for idx in 0..signed_tx.chunks.len() {
            let chunk = signed_tx.get_chunk(idx)?;
            let chunk_resp = self.post_chunk(chunk).await?;
            if chunk_resp.status() != reqwest::StatusCode::OK {
                error!("Arweave upload failed, due to {}", chunk_resp.text().await?);
                return Err(Error::StatusCodeNotOk)
            }
        }
        Ok((signed_tx.get_id()?, signed_tx.get_reward()?))
    }

    pub async fn upload_file(&mut self, data: Vec<u8>, content_type: Option<&str>) -> Result<String> {
        let mut tx = self.create_transaction(data, content_type).await?;

        self.sign_transaction(&mut tx)?;

        let (id, reward) = self.post_transaction(tx).await?;
        let tx_addr = self.make_endpoint(&id);

        info!("Successfully uploaded game bundle to {}", tx_addr);
        info!("Paid {} Winstons for the transaction", reward);

        Ok(tx_addr)
    }
}
