use std::str::FromStr;

use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::error::Result;

use race_core::types::{
    CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameRegistration,
    RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams, ServerAccount,
    UnregisterGameParams, VoteParams,
};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, SettleParams,
    },
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::create_account;
use solana_sdk::transaction::Transaction;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub addr: Pubkey,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
    pub padding: Box<Vec<u8>>,
}

pub struct SolanaTransport {
    rpc: String,
}

const KEYPAIR: [u8; 64] = [
    56, 64, 63, 204, 174, 191, 176, 251, 217, 61, 160, 59, 37, 187, 86, 237, 61, 242, 73, 191, 172,
    249, 27, 219, 185, 179, 141, 95, 113, 122, 222, 101, 62, 68, 46, 235, 154, 225, 125, 94, 44,
    220, 213, 92, 25, 2, 123, 208, 217, 155, 7, 172, 15, 30, 193, 126, 196, 118, 177, 73, 124, 7,
    254, 242,
];
const PROGRAM_ID: &str = "DY2KTjFAT89KqbTZarvDLYEaaQJh1qSBV4kNAxB5KXbH";

#[async_trait]
#[allow(unused_variables)]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        Ok("".into())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        todo!()
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        todo!()
    }
    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        Ok("".into())
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let client = RpcClient::new(&self.rpc);
        let payer = Keypair::from_bytes(&KEYPAIR).unwrap();
        let payer_pubkey = payer.pubkey();
        let registry_account = Keypair::new();
        let registry_account_pubkey = registry_account.pubkey();
        let registry_data_len = 2000;
        let lamports = client
            .get_minimum_balance_for_rent_exemption(registry_data_len)
            .unwrap();
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        let create_account_ix = create_account(
            &payer_pubkey,
            &registry_account_pubkey,
            lamports,
            registry_data_len as _,
            &program_id,
        );
        let create_registry_ix = Instruction::new_with_bytes(
            program_id,
            &[2],
            vec![
                AccountMeta {
                    pubkey: payer_pubkey,
                    is_signer: true,
                    is_writable: false,
                },
                AccountMeta {
                    pubkey: registry_account_pubkey,
                    is_signer: false,
                    is_writable: true,
                },
            ],
        );

        let message = Message::new(
            &[create_account_ix, create_registry_ix],
            Some(&payer.pubkey()),
        );
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = client.get_latest_blockhash().unwrap();
        tx.sign(&[&payer, &registry_account], blockhash);
        client.send_and_confirm_transaction(&tx).unwrap();
        Ok(registry_account_pubkey.to_string())
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let client = RpcClient::new(&self.rpc);
        let registry_account_pubkey = Pubkey::from_str(addr).unwrap();
        let data = client.get_account_data(&registry_account_pubkey).unwrap();
        let state = RegistryState::try_from_slice(&data).unwrap();
        Some(RegistrationAccount {
            addr: addr.to_owned(),
            is_private: true,
            size: 100,
            owner: Some(state.owner.to_string()),
            games: state.games.into_iter().map(|g| GameRegistration {
                title: "".into(),
                addr: "".into(),
                reg_time: 0,
                bundle_addr: "".into(),
            }).collect(),
        })
    }
}

impl SolanaTransport {
    pub fn new(rpc: String) -> Self {
        Self { rpc }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_registration() -> anyhow::Result<()> {
        let transport = SolanaTransport::new("http://localhost:8899".into());
        let addr = transport
            .create_registration(CreateRegistrationParams {
                is_private: true,
                size: 100,
            })
            .await?;

        let reg = transport.get_registration(&addr).await.unwrap();
        assert_eq!(reg.addr, addr);
        Ok(())
    }
}
