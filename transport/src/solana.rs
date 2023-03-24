#![allow(unused_variables, unused_imports)]
use crate::error::{TransportError, TransportResult};
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    error::Result,
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration,
        JoinParams, PlayerJoin, PlayerProfile, RegisterGameParams, RegisterServerParams,
        RegistrationAccount, ServeParams, ServerAccount, ServerJoin, SettleParams,
        UnregisterGameParams, VoteParams,
    },
};

use race_solana_types::error::InstructionError;
use race_solana_types::instruction::RaceInstruction;
use race_solana_types::types as solana_types;

use serde_json;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::create_account;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    check_id, check_program_account, id, instruction::initialize_account, state::Account, ID,
};

// TODO: Move the following structs to a separate module
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

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct Player {
    pub addr: Pubkey,
    pub balance: u64,
    pub position: u32,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct Server {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct TokenInfo {
    pub pubkey: Pubkey,
    pub token: String,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct GameState {
    pub is_initialized: bool,
    pub title: String,
    pub bundle_addr: Pubkey,
    pub stake_addr: Pubkey, // stake account address, to be replaced by TokenInfo?
    pub owner: Pubkey,
    pub transactor_addr: Option<Pubkey>,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u8,
    pub data_len: u32,
    pub data: Box<Vec<u8>>,
    pub players: Box<Vec<Player>>,
    pub servers: Box<Vec<Server>>,
    pub padding: Box<Vec<u8>>,
}

fn read_keypair(path: PathBuf) -> TransportResult<Keypair> {
    let keypair = solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| TransportError::InvalidKeyfile(e.to_string()))?;
    Ok(keypair)
}

const PROGRAM_ID: &str = "8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu";
// TODO: move token consts to config file and import it for program
const SOL: &str = "So11111111111111111111111111111111111111112";

pub struct SolanaTransport {
    client: RpcClient,
    keypair: Keypair,
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        let program_id = Pubkey::from_str(PROGRAM_ID).map_err(|_| TransportError::InvalidConfig)?;
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let bundle_pubkey = Pubkey::from_str(&params.bundle_addr)
            .map_err(|_| TransportError::InvalidBundleAddress)?;
        let game_account = Keypair::new();
        let game_account_pubkey = game_account.pubkey();
        let game_account_len = 5000;
        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(game_account_len)
            .map_err(|_| TransportError::NoEnoughLamports)
            .unwrap();
        let create_game_account_ix = create_account(
            &payer_pubkey,
            &game_account_pubkey,
            lamports,
            game_account_len as u64,
            &program_id,
        );

        let token_pubkey = Pubkey::from_str(SOL).unwrap();
        let stake_account = Keypair::new();
        let stake_account_pubkey = stake_account.pubkey();
        let stake_account_len = Account::LEN;
        let stake_lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(stake_account_len)
            .map_err(|_| TransportError::NoEnoughLamports)
            .unwrap();
        let create_temp_account_ix = create_account(
            &payer_pubkey,
            &stake_account_pubkey,
            stake_lamports,
            stake_account_len as u64,
            &ID,
        );

        let init_temp_account_ix =
            initialize_account(&ID, &stake_account_pubkey, &token_pubkey, &payer_pubkey)
                .map_err(|e| InstructionError::InitAccountFailed)
                .unwrap();

        // FIXME: limit title to 16 or 30 chars
        let ix_data = RaceInstruction::pack(RaceInstruction::CreateGameAccount {
            params: solana_types::CreateGameAccountParams {
                title: params.title,
                max_players: params.max_players,
                data: params.data,
            },
        })
        .map_err(|_| TransportError::InstructionDataError)?;

        let create_game_ix = Instruction::new_with_bytes(
            program_id.clone(),
            &ix_data,
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, true),
                AccountMeta::new_readonly(token_pubkey, false),
                AccountMeta::new_readonly(ID, false),
                AccountMeta::new_readonly(bundle_pubkey, false),
                // TODO: add or impl scene pubkey
            ],
        );
        let message = Message::new(
            &[
                create_game_account_ix,
                create_temp_account_ix,
                init_temp_account_ix,
                create_game_ix,
            ],
            Some(&payer.pubkey()),
        );
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer, &game_account, &stake_account], blockhash);
        self.client
            .send_and_confirm_transaction(&tx)
            .map_err(|_| TransportError::ClientSendTransactionFailed)?;

        Ok(game_account_pubkey.to_string())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        // payer is initializer/owner of the to-be-closed game
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let program_id = Pubkey::from_str(PROGRAM_ID).map_err(|_| TransportError::InvalidConfig)?;

        let game_account_pubkey = Pubkey::from_str(&params.addr)
            .map_err(|_| TransportError::InvalidPubkey(params.addr))?;
        let game_account_data = &self
            .client
            .get_account_data(&game_account_pubkey)
            .map_err(|_| TransportError::InvalidPubkey(game_account_pubkey.to_string()))?;
        let game_state = GameState::try_from_slice(&game_account_data)?;
        let stake_account_pubkey = game_state.stake_addr.clone();

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &program_id);

        let ix_data = RaceInstruction::pack(RaceInstruction::CloseGameAccount)
            .map_err(|_| TransportError::InstructionDataError)?;

        let close_game_ix = Instruction::new_with_bytes(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new_readonly(pda, false),
                AccountMeta::new_readonly(ID, false),
            ],
        );

        // println!("{:?}", payer_pubkey);
        // println!("{:?}", game_account_pubkey);
        // println!("{:?}", stake_account_pubkey);
        // println!("{:?}", pda);
        // println!("{:?}", ID);

        let message = Message::new(&[close_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);

        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);
        // TODO: move this to test
        let config = RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        };
        let signature = &self
            .client
            .send_transaction_with_config(&tx, config)
            .map_err(|_| TransportError::ClientSendTransactionFailed)?;

        // self.client
        //     .send_and_confirm_transaction(&tx)
        //     // .map_err(|_| TransportError::ClientSendTransactionFailed)?;
        //     .unwrap();
        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        // Create server profile on chain (like creation of a player profile)
        // server prof: endpoint (URL)
        Ok("".into())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        // Player Join
        // 1. check max player (must be <= max player)
        // 2. check position 0 to max_ply - 1
        // 3. access_version == current access_version; if different, fail
        // each
        todo!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        // Server joins a game
        // 1. check Max server num (use a const)
        // 2. access_v += 1
        // 3. 1st-joine server becomes transactor (if there is no one currently)
        todo!()
    }
    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        let player = &self.keypair;
        let player_pubkey = player.pubkey();
        let balance = self
            .client
            .get_balance(&player_pubkey)
            .map_err(|_| TransportError::InvalidBalance(player_pubkey.to_string()))
            .unwrap();

        // TODO: Add Racetoken ATA

        let profile_account = Keypair::new();
        let profile_account_pubkey = profile_account.pubkey();
        // Limit nick name to 30 chars
        // let profile_account_len =
        todo!()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        // Publish game bundle (similar to minting NFTs)
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        // TODO: test fn settle with non-trans add failed
        todo!()
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let registry_account = Keypair::new();
        let registry_account_pubkey = registry_account.pubkey();
        let registry_data_len = 2000;
        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(registry_data_len)
            .map_err(|_| TransportError::NoEnoughLamports)
            .unwrap();
        let program_id = Pubkey::from_str(PROGRAM_ID)
            .map_err(|_| TransportError::InvalidConfig)
            .unwrap();
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
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)
            .unwrap();
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&payer, &registry_account], blockhash);
        self.client
            .send_and_confirm_transaction(&tx)
            .map_err(|_| TransportError::ClientSendTransactionFailed)?;
        Ok(registry_account_pubkey.to_string())
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let game_account_pubkey = Pubkey::from_str(addr)
            .map_err(|_| TransportError::InvalidConfig)
            .unwrap();
        let data = self
            .client
            .get_account_data(&game_account_pubkey)
            .map_err(|_| TransportError::ClientGetDataFailed)
            .unwrap();
        let state = GameState::try_from_slice(&data)
            .map_err(|_| TransportError::ClientGetDataFailed)
            .unwrap();
        let bundle_addr = state.bundle_addr.to_string();
        let transactor_addr = match state.transactor_addr {
            Some(pubkey) => Some(pubkey.to_string()),
            None => None,
        };

        Some(GameAccount {
            addr: addr.to_owned(),
            title: state.title,
            settle_version: state.settle_version,
            bundle_addr,
            access_version: state.access_version,
            players: state
                .players
                .into_iter()
                .map(|p| PlayerJoin {
                    addr: p.addr.to_string(),
                    position: p.position as usize,
                    balance: p.balance,
                    access_version: p.access_version,
                })
                .collect(),
            servers: state
                .servers
                .into_iter()
                .map(|s| ServerJoin {
                    addr: s.addr.to_string(),
                    endpoint: s.endpoint,
                    access_version: s.access_version,
                })
                .collect(),
            transactor_addr,
            max_players: state.max_players,
            data_len: state.data_len,
            data: *state.data,
            // TODO: impl the following fields
            deposits: Vec::new(),
            votes: Vec::new(),
            unlock_time: None,
        })
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        match Pubkey::from_str(addr) {
            Ok(game_account_pubkey) => {
                match &self.client.get_account_data(&game_account_pubkey) {
                    Ok(game_account_data) => {
                        match GameState::try_from_slice(&game_account_data) {
                            Ok(game_state) => {
                                // FIXME: implement GameBundle as NFT and use its metadata
                                let bundle_addr = game_state.bundle_addr.to_string();
                                let bundle_data = "ARWEAVE BASE64 ADDRESS".to_string();

                                Some(GameBundle {
                                    addr: bundle_addr,
                                    data: bundle_data,
                                })
                            }
                            Err(e) => {
                                eprintln!("Game State Error: {}",e);
                                None
                            },
                        }
                    }
                    Err(e) => {
                        eprintln!("Game Account Data Error {}",e);
                        None
                    },
                }
            }
            Err(e) => {
                eprintln!("Game Account Pubkey Error {}",e);
                None
            },
        }
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        todo!()
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let registry_account_pubkey = Pubkey::from_str(addr).unwrap();
        let data = self
            .client
            .get_account_data(&registry_account_pubkey)
            .unwrap();
        let state = RegistryState::try_from_slice(&data).unwrap();
        Some(RegistrationAccount {
            addr: addr.to_owned(),
            is_private: true,
            size: 100,
            owner: Some(state.owner.to_string()),
            games: state
                .games
                .into_iter()
                .map(|g| GameRegistration {
                    title: "".into(),
                    addr: "".into(),
                    reg_time: 0,
                    bundle_addr: "".into(),
                })
                .collect(),
        })
    }
}

impl SolanaTransport {
    pub fn try_new(rpc: String, keyfile: PathBuf) -> TransportResult<Self> {
        let client = RpcClient::new(rpc);

        let keypair = read_keypair(keyfile)?;
        Ok(Self { client, keypair })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_keypair() -> anyhow::Result<()> {
        let keypair = read_keypair(
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
        )?;
        Ok(())
    }

    fn get_transport() -> anyhow::Result<SolanaTransport> {
        let transport = SolanaTransport::try_new(
            "http://localhost:8899".into(),
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
        )?;
        Ok(transport)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_registration() -> anyhow::Result<()> {
        let transport = get_transport()?;
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "HHHHH".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                max_players: 9,
                data: Vec::<u8>::new(),
            })
            .await?;

        let game = transport.get_game_account(&addr).await.unwrap();
        assert_eq!(game.addr, addr);
        assert_eq!(
            game.bundle_addr,
            "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_string()
        );
        assert_eq!(game.title, "HHHHH".to_string());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_close_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "HHHHH".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                max_players: 9,
                data: Vec::<u8>::new(),
            })
            .await?;
        // 6oMGzhGRo8qUm3eyW7WpVL3j4kXF1eSJp4ag4MzPt4c6

        // let addr = "Mr9bzxPUP88eeYQxfzuqsdyZuznu7mfp4t2QxqhKnVR".to_string();
        println!("To close game account {}", addr);
        transport
            .close_game_account(CloseGameAccountParams { addr })
            .await?;

        Ok(())
    }
}
