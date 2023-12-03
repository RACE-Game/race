//! The API module of this crate which is used to construct the needed json file
//! for uploading files to Awearve using its HTTP API
//! This mod corresponds to the `common/transactions.ts` module of arweave-js
use reqwest::{
    self,
    header::{ACCEPT, CONTENT_TYPE},
    Client, Response, StatusCode,
};
use tokio::time::{sleep, Duration};

use crate::constants::*;
use crate::crypto::{self, ArweaveKey};
use crate::error::{Error, Result};
use crate::metadata::Metadata;
use crate::transaction::{Chunk, Transaction};
use serde_json;
use std::fs;
use std::path::PathBuf;

/// Struct with methods for interacting with the Arweave network.
pub struct Arweave {
    pub base_url: String,
    pub arkey: ArweaveKey,
}

impl Arweave {
    pub fn new(keypath: Option<&str>) -> Result<Self> {
        if let Some(kp) = keypath {
            Ok(Self {
                base_url: String::from("https://arweave.net/"),
                arkey: ArweaveKey::new_from_file(kp)?,
            })
        } else {
            Ok(Self {
                base_url: String::from("https://arweave.net/"),
                arkey: ArweaveKey::new_from_file(AR_KEYFILE_PATH)?,
            })
        }
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
    pub async fn create_transaction(&self, data: Vec<u8>, json: bool) -> Result<Transaction> {
        let mut tx = Transaction::new();

        let reward = self.get_price(data.len()).await?;
        let last_tx = self.get_last_tx().await?;
        tx.set_reward(reward)?;
        tx.set_last_tx(&last_tx)?;

        tx.set_data(data, json)?;

        Ok(tx)
    }

    pub fn sign_transaction(&self, tx: &mut Transaction) -> Result<()> {
        tx.set_owner(&self.arkey.get_modulus()?)?;
        let deephash = tx.get_deephash()?;
        let signature = self.arkey.sign(&deephash)?;
        println!("== Signature length {}", signature.len());
        println!(
            "== Signature base64url {:?}",
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
        println!("== Post chunks to {}", url);
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
        println!("tx json body {}", json_tx);

        let mut status = StatusCode::NOT_FOUND;
        let url = self.make_endpoint("tx");
        println!("== Post to {}", url);

        let client = Client::new();
        let resp = client
            .post(&url)
            .header(&CONTENT_TYPE, "application/json")
            .header(&ACCEPT, "application/json text/plain, */*")
            .body(json_tx.clone())
            .send()
            .await?;
        status = resp.status();
        let body = resp.text().await?;
        println!("== response body: {:?}", body);

        for idx in 0..signed_tx.chunks.len() {
            let chunk = signed_tx.get_chunk(idx)?;
            let chunk_resp = self.post_chunk(chunk).await?;
            let ck_resp_body = chunk_resp.text().await?;
            println!("== response body: {:?}", ck_resp_body);
        }

        if status == reqwest::StatusCode::OK {
            println!("== posted transaction: {:?}", status);
            return Ok((signed_tx.get_id()?, signed_tx.get_reward()?));
        }
        sleep(Duration::from_secs(10)).await;

        Err(Error::StatusCodeNotOk)
    }

    pub async fn upload_file(&mut self, data: Vec<u8>, json: bool) -> Result<String> {
        let mut tx = self.create_transaction(data, json).await?;
        println!("== Created transaction for the given file");

        self.sign_transaction(&mut tx)?;

        let (id, reward) = self.post_transaction(tx).await?;
        let tx_addr = self.make_endpoint(&id);
        println!("== Successfully uploaded game bundle to {}", tx_addr);
        println!("== Paid {} Winstons for the  transaction", reward);

        Ok(tx_addr)
    }

    // Metadata
    pub fn read_metadata_file(&self, path: PathBuf) -> Result<Metadata> {
        let data = fs::read_to_string(path)?;
        let metadata: Metadata = serde_json::from_str(&data)?;
        Ok(metadata)
    }

    pub fn update_metadata(
        &self,
        name: String,
        symbol: String,
        file_addr: String,
        mime: String,
        creator_addr: String,
        meta: &mut Metadata,
    ) -> Result<()> {
        if name.len() > MAX_NAME_LENGTH {
            return Err(Error::InvalidNameLength);
        }

        if symbol.len() > MAX_SYMBOL_LENGTH {
            return Err(Error::InvalidSymbolLength);
        }

        meta.name = name;
        meta.symbol = symbol;
        meta.add_file(file_addr, mime)?;
        meta.add_creator(creator_addr)?;
        Ok(())
    }

    /// Publish game bundle, used by race-cli `publish` command
    pub async fn publish_game(
        &mut self,
        name: String,
        symbol: String,
        creator: String,
        bundle: String,
    ) -> Result<String> {
        println!("== bundle {}", bundle);
        println!("== creator {}", creator);
        let data = fs::read(PathBuf::from(&bundle))?;
        let bundle_addr = self.upload_file(data, false).await?;

        let metadata = Metadata::new(name, symbol, creator, bundle_addr)?;
        let json_meta = serde_json::to_vec(&metadata)?;

        // Start uploading the metadata.json
        let meta_addr = self.upload_file(json_meta, true).await?;

        Ok(meta_addr)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::constants::{WASM1, WASM2};
    use crate::crypto;
    use serde_json;
    use tokio::fs;

    // Create a transaction struct using the data from successful txs
    async fn make_test_transaction(path: &str) -> anyhow::Result<Transaction> {
        let arweave = Arweave::new(None)?;
        let data = fs::read(path).await?;
        let mut tx = Transaction::new();
        let reward = arweave.get_price(data.len()).await?;
        tx.set_reward(reward)?;
        tx.set_data(data, false)?; // data_size, data_root, data, tags
        let owner = "g1gL9QEVZ6yIXqom8ZFhkFfszVi2F9rZ1_oUFZQPSTAqu3QjECWxnkQgb9SQM7REFZJGX21LnZenPBaIeFay2S9_WYVvQEqjkxKPMnFE04i-q7qWetDyolzaElRdL8IvN4BG1nVePeWi1Z3-3aVjaat_p65LNdgaZ9heYyMnFq6XLfspLbfaa6_BNyzZjz6F-ME9ro8TDNgd3as-vmdhvTh3QNJqGWg6CGxkyBIPoCRVXw9ADvl-OAhgStpJJPVqo7wvp6teWTYu33JFyFadzkhU1s3oyIp4Np9tBYs6C96VwuT_0clUKSIb6f2CC__eClt3-aejmPrmTRS6Qhbhp3WhU5KRhvF7L-ya1AhgP_jmpnJTovhjjHQL9vY74lQfhN6M_SGvSchAJQd4bTkQf6x9tmEedKkZfK-ntA45uVD1LW3WPHYqIIeo2cBuaEbwK_csYgjVXNKym0guLgGNYVpAjSPLo7Eu1BFDbe0Gc8d0GOR4p7HaZf4X6udIP5ypF1bGlVDgCSSfYiSDAW5xv61_BPoXukVzoC7C6aP4OXz4p_9naUIce77SEbt19GOZg_9KZAUmtgZOxgsRm1nvyXiyBc2h87JF4KnSA1PJq4EMUsD3pt9vE2Uc9IZ9-7fOiycKYLFlXMVyhURjNCAYZA1sVVJXTWDP7mSoyEQAiqE";
        tx.set_owner(&crypto::b64_decode(owner)?)?;
        let last_tx = arweave.get_last_tx().await?;
        tx.set_last_tx(&last_tx)?;

        Ok(tx)
    }

    #[tokio::test]
    async fn test_transaction_json() -> anyhow::Result<()> {
        let mut tx = make_test_transaction(WASM2).await.unwrap();
        let arweave = Arweave::new(None)?;
        arweave.sign_transaction(&mut tx).unwrap();
        let json_tx = serde_json::to_string(&tx).expect("Error serializing struct to JSON");

        // Specify the file path
        let file_path = "tests/transaction.json";

        // Write the JSON string to the file
        std::fs::write(file_path, json_tx).expect("Error writing JSON to file");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_balance() -> anyhow::Result<()> {
        let arweave = Arweave::new(None)?;
        let balance = arweave.get_balance().await?;
        println!("Wallet balance in Winston: {}", balance);
        // println!("Wallet balance in AR: {}", balance / 1_000_000_000_000);
        assert_eq!(balance, 1_581_480_759_804,);
        // 1_582_323_701_608
        // assert_eq!(balance, 1_582_000_000_000);

        Ok(())
    }

    #[tokio::test]
    async fn test_upload_file() -> anyhow::Result<()> {
        // let path = "tests/garbage.wasm";
        let data = fs::read(WASM2).await?;
        let mut arweave = Arweave::new(None)?;
        let _txid = arweave.upload_file(data, false).await?;
        println!("== tx id {}", _txid);
        Ok(())
    }

    #[tokio::test]
    // The provided data_path and offsets in this test come from
    // chunking the same data using arweave-js
    async fn test_chunks() -> anyhow::Result<()> {
        let tx = make_test_transaction(WASM2).await.unwrap();
        let data_root = crypto::b64_encode(&tx.data_root)?;
        assert_eq!(tx.chunks.len(), 3);
        for idx in 0..tx.chunks.len() {
            let chunk = tx.get_chunk(idx)?;
            let chunk_data_root = crypto::b64_encode(&chunk.data_root)?;
            // data_root: cECmgOI8BpquDp_OjfZtnLOZ1dCC7lqpm4XDK78hMC4
            assert_eq!(data_root, chunk_data_root);
            // data_size: 524722
            assert_eq!(tx.data_size, chunk.data_size);
            let data_path = crypto::b64_encode(&chunk.data_path)?;
            let proof_offset = chunk.offset;
            match idx {
                0 => {
                    assert_eq!(&data_path,
                               "lPfRnG2YWTaF7WnEZktL2k9FoDpDAZl4FGt_iIwIBbXOxiLSNa0c6ccRmvYgYAOG_9_c6MQPg_MF20G5FNeqQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgDZEVcYn0t5H0OvNTCvsw9JH1F07ZDs9HFi8aXmN-ggnNgF4rpl7230jLQtYaM_cBCu9wA0jVygq0PtDLdakV08EwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAABpGYsYl3tiVBtltNZiF4Ww7HpE05WcDdYZkqQRIp0SwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAA");
                    assert_eq!(&proof_offset, "262143");
                }

                1 => {
                    assert_eq!(&data_path,
        "lPfRnG2YWTaF7WnEZktL2k9FoDpDAZl4FGt_iIwIBbXOxiLSNa0c6ccRmvYgYAOG_9_c6MQPg_MF20G5FNeqQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgDZEVcYn0t5H0OvNTCvsw9JH1F07ZDs9HFi8aXmN-ggnNgF4rpl7230jLQtYaM_cBCu9wA0jVygq0PtDLdakV08EwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAKAboMOyTKP3JHNb3MwOA2syD9fvn0b7yICUrff0vWxoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAYA2Q");
                    assert_eq!(&proof_offset, "393432");
                }

                2 => {
                    assert_eq!(&data_path,
        "lPfRnG2YWTaF7WnEZktL2k9FoDpDAZl4FGt_iIwIBbXOxiLSNa0c6ccRmvYgYAOG_9_c6MQPg_MF20G5FNeqQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgDZuArKIn0QQppYOX-AC4fvGHupxRXI8x2NbBIzRIxwa0IAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgBsg");
                    assert_eq!(&proof_offset, "524721");
                }

                _ => panic!("expected to have 3 chunks only"),
            }
        }

        Ok(())
    }
}
