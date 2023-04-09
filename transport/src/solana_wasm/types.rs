use crate::{error::TransportResult, wasm_utils::*};
use borsh::BorshDeserialize;
use gloo::console::warn;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

fn get_sol() -> Object {
    let window = gloo::utils::window();
    window.get("solanaWeb3").unwrap()
}

pub(crate) struct Pubkey {
    pub(crate) value: JsValue,
}

impl Pubkey {
    pub fn new(addr: &str) -> Self {
        let pubkey_ctor = get_function(&get_sol(), "PublicKey");
        let value = construct(&pubkey_ctor, &[&addr.clone().into()]);
        Self { value }
    }

    pub fn to_base58(&self) -> String {
        let f = get_function(&self.value, "toBase58");
        f.call0(&self.value).unwrap().as_string().unwrap()
    }
}

pub(crate) struct Keypair {
    value: JsValue,
}

impl Keypair {
    pub fn new() -> Self {
        let keypair = rget(&get_sol(), "Keypair");
        let f = get_function(&keypair, "generate");
        let value = f.call0(&JsValue::undefined()).unwrap();
        Self { value }
    }

    pub fn public_key(&self) -> Pubkey {
        let f = get_function(&self.value, "publicKey");
        let value = f.call0(&self.value).unwrap();
        Pubkey { value }
    }
}

pub(crate) struct Connection {
    value: JsValue,
}

impl Connection {
    pub fn new(rpc: &str) -> Self {
        let rpc = JsValue::from_str(rpc);
        let conn_ctor = get_function(&get_sol(), "Connection");
        let value = construct(&conn_ctor, &[&rpc]);
        Self { value }
    }

    pub fn get_latest_blockhash(&self) -> JsValue {
        let f = get_function(&self.value, "getLatestBlockhashAndContext");
        let v = f.call0(&self.value).unwrap();
        let value = rget(&v, "value");
        let blockhash = rget(&v, "blockhash");
        blockhash
    }

    pub fn get_minimum_balance_for_rent_exemption(&self, len: usize) -> u64 {
        let f = get_function(&self.value, "getMinimumBalanceForRentExemption");
        let v = f.call1(&self.value, &len.into()).unwrap().as_f64().unwrap();
        v as u64
    }

    pub async fn send_transaction(&self) -> Signature {
        let serialize = get_function(&self.value, "serialize");
        let serialized = serialize.call0(&self.value).unwrap();
        let f = get_function(&self.value, "sendRawTransaction");
        let sig_p = f.call1(&self.value, &serialized).unwrap();
        let sig = resolve_promise(sig_p).await.unwrap();
        Signature { value: sig }
    }

    pub async fn get_account_state<T: BorshDeserialize>(&self, pubkey: &Pubkey) -> Option<T> {
        let data = self.get_account_data(pubkey).await?;
        T::try_from_slice(&data).ok()
    }

    pub async fn get_account_data(&self, pubkey: &Pubkey) -> Option<Vec<u8>> {
        let get_account_info = get_function(&self.value, "getAccountInfo");
        let p = match get_account_info.call1(&self.value, &pubkey.value) {
            Ok(p) => p,
            Err(e) => {
                warn!("Error when getting account data", e);
                return None;
            }
        };
        let account_info = resolve_promise(p).await?;
        let data = match Reflect::get(&account_info, &"data".into()) {
            Ok(d) => d,
            Err(e) => {
                warn!("Error when getting account data, promise error", e);
                return None;
            }
        };

        let data = match data.dyn_into::<Uint8Array>() {
            Ok(d) => d,
            Err(e) => {
                warn!("Error when getting account data, promise error", e);
                return None;
            }
        };
        Some(data.to_vec())
    }
}

pub(crate) struct Transaction {
    value: JsValue,
}

impl Transaction {
    pub fn new(conn: &Connection, payer_pubkey: &Pubkey) -> Self {
        let transaction_ctor = get_function(&get_sol(), "Transaction");
        let blockhash = conn.get_latest_blockhash();
        let params = create_object(&[
            ("recentBlockhash", &blockhash),
            ("feePayer", &payer_pubkey.value),
        ]);
        let transaction = construct(&transaction_ctor, &[&params]);
        Self { value: transaction }
    }

    pub fn add(&self, ix: &Instruction) {
        let f = get_function(&self.value, "add");
        f.call1(&self.value, &ix.value).unwrap();
    }
}

pub(crate) struct Instruction {
    pub(crate) value: JsValue,
}

impl Instruction {
    pub fn create_account(
        from_pubkey: &Pubkey,
        new_account_pubkey: &Pubkey,
        lamports: u64,
        space: usize,
    ) -> Self {
        let system_instruction = rget(&get_sol(), "SystemInstruction");
        let f = get_function(&system_instruction, "createAccount");
        let params = create_object(&[
            ("fromPubkey", &from_pubkey.value),
            ("newAccountPubkey", &new_account_pubkey.value),
            ("lamports", &lamports.into()),
            ("space", &space.into()),
        ]);
        let value = f.call1(&JsValue::undefined(), &params).unwrap();
        Self { value }
    }
}

pub(crate) struct Signature {
    pub(crate) value: JsValue,
}
