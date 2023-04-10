use crate::{
    error::{TransportError, TransportResult},
    wasm_utils::*,
};
use borsh::{BorshDeserialize, BorshSerialize};
use gloo::console::{debug, error, info, warn};
use js_sys::{Array, ArrayBuffer, Object, Reflect, Uint8Array};
use race_solana_types::constants::PROFILE_SEED;
use wasm_bindgen::{JsCast, JsValue};

fn get_sol() -> Object {
    let window = gloo::utils::window();
    window.get("solanaWeb3").unwrap()
}

pub(crate) struct Pubkey {
    pub(crate) value: JsValue,
}

impl Pubkey {
    pub fn try_new(addr: &str) -> TransportResult<Self> {
        let pubkey_ctor = get_function(&get_sol(), "PublicKey");
        if let Ok(value) = construct(&pubkey_ctor, &[&addr.clone().into()]) {
            Ok(Self { value })
        } else {
            Err(TransportError::InvalidPubkey(addr.to_owned()))
        }
    }

    pub async fn create_with_seed(from_pubkey: &Pubkey, seed: &str, program_id: &Pubkey) -> Self {
        let pubkey = rget(&get_sol(), "PublicKey");
        let f = get_function(&pubkey, "createWithSeed");
        let value_p = f
            .call3(
                &JsValue::undefined(),
                &from_pubkey.value,
                &JsValue::from_str(seed),
                &program_id.value,
            )
            .unwrap();
        let value = resolve_promise(value_p).await.unwrap();
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
        let value = construct(&conn_ctor, &[&rpc]).unwrap();
        Self { value }
    }

    pub fn get_latest_blockhash_and_context(&self) -> JsValue {
        let f = get_function(&self.value, "getLatestBlockhashAndContext");
        f.call0(&self.value).unwrap()
    }

    pub fn get_latest_blockhash(&self) -> JsValue {
        let f = get_function(&self.value, "getLatestBlockhashAndContext");
        let v = f.call0(&self.value).unwrap();
        let value = rget(&v, "value");
        let blockhash = rget(&v, "blockhash");
        blockhash
    }

    pub async fn get_minimum_balance_for_rent_exemption(&self, len: usize) -> u32 {
        let f = get_function(&self.value, "getMinimumBalanceForRentExemption");
        let v_p = f.call1(&self.value, &len.into()).unwrap();
        let v = resolve_promise(v_p).await.unwrap();
        v.as_f64().unwrap() as u32
    }

    pub async fn send_transaction(&self, wallet: &JsValue, tx: &Transaction) -> Signature {
        let blockhash_and_context_p = self.get_latest_blockhash_and_context();
        let blockhash_and_context = resolve_promise(blockhash_and_context_p).await.unwrap();
        let context = rget(&blockhash_and_context, "context");
        let min_context_slot = rget(&context, "slot");
        debug!("min context slot:", &min_context_slot);
        let f = get_function(wallet, "sendTransaction");
        let send_opts = create_object(&[("minContextSlot", &min_context_slot)]);
        debug!(&tx.value);
        let sig_p = f
            .call3(&wallet, &tx.value, &self.value, &send_opts)
            .unwrap();
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
        let transaction = construct(&transaction_ctor, &[&params]).unwrap();
        // let transaction = construct(&transaction_ctor, &[]);
        Self { value: transaction }
    }

    pub fn add(&self, ix: &Instruction) {
        let f = get_function(&self.value, "add");
        f.call1(&self.value, &ix.value).unwrap();
    }

    pub fn serialize(&self) -> JsValue {
        let f = get_function(&self.value, "serialize");
        f.call0(&self.value).unwrap()
    }

    pub fn partial_sign(&self, signers: &[&Pubkey]) {
        let f = get_function(&self.value, "partialSign");
        let args = Array::new();
        for signer in signers.iter() {
            args.push(&signer.value);
        }
        f.apply(&self.value, &args).unwrap();
    }
}

pub(crate) struct Instruction {
    pub(crate) value: JsValue,
}

impl Instruction {
    pub fn new_with_borsh<T: BorshSerialize>(
        program_id: &Pubkey,
        ix_data: T,
        account_metas: Vec<AccountMeta>,
    ) -> Self {
        let ctor = get_function(&get_sol(), "TransactionInstruction");
        let data_vec = ix_data.try_to_vec().unwrap();
        let data = Uint8Array::new_with_length(data_vec.len() as _);
        data.copy_from(&data_vec);
        let utils = rget(&get_sol(), "utils");
        let keys_arr = Array::new();
        for account_meta in account_metas.iter() {
            keys_arr.push(&account_meta.value);
        }
        let opts = create_object(&[
            ("keys", &keys_arr),
            ("programId", &program_id.value),
            ("data", &data),
        ]);
        let value = construct(&ctor, &[&opts]).unwrap();
        Self { value }
    }

    pub fn create_account(
        from_pubkey: &Pubkey,
        new_account_pubkey: &Pubkey,
        lamports: u32,
        space: usize,
        program_id: &Pubkey,
    ) -> Self {
        let system_program = rget(&get_sol(), "SystemProgram");
        let f = get_function(&system_program, "createAccount");
        let params = create_object(&[
            ("fromPubkey", &from_pubkey.value),
            ("newAccountPubkey", &new_account_pubkey.value),
            ("lamports", &lamports.into()),
            ("space", &space.into()),
            ("programId", &program_id.value),
        ]);
        let value = f
            .call1(&system_program, &params)
            .map_err(|e| error!(e))
            .unwrap();
        Self { value }
    }

    pub fn create_account_with_seed(
        from_pubkey: &Pubkey,
        new_account_pubkey: &Pubkey,
        base_pubkey: &Pubkey,
        seed: &str,
        lamports: u32,
        space: usize,
        program_id: &Pubkey,
    ) -> Instruction {
        let system_program = rget(&get_sol(), "SystemProgram");
        let f = get_function(&system_program, "createAccountWithSeed");
        let params = create_object(&[
            ("fromPubkey", &from_pubkey.value),
            ("newAccountPubkey", &new_account_pubkey.value),
            ("basePubkey", &base_pubkey.value),
            ("seed", &seed.into()),
            ("lamports", &lamports.into()),
            ("space", &space.into()),
            ("programId", &program_id.value),
        ]);
        let value = f
            .call1(&system_program, &params)
            .map_err(|e| error!(e))
            .unwrap();
        Self { value }
    }
}

pub(crate) struct Signature {
    pub(crate) value: JsValue,
}

pub(crate) struct AccountMeta {
    pub(crate) value: Object,
}

impl AccountMeta {
    pub fn new(pubkey: &Pubkey, is_signer: bool, is_writable: bool) -> Self {
        let value = create_object(&[
            ("pubkey", &pubkey.value),
            ("isSigner", &JsValue::from_bool(is_signer)),
            ("isWritable", &JsValue::from_bool(is_writable)),
        ]);
        Self { value }
    }

    pub fn new_readonly(pubkey: &Pubkey, is_signer: bool) -> Self {
        Self::new(pubkey, is_signer, false)
    }
}
