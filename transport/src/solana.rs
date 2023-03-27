#![allow(unused_variables, unused_imports)]
use crate::error::{TransportError, TransportResult};
use crate::states::{GameReg, RegistryState, GameState, PlayerState, ServerState};
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use jsonrpsee::core::Error;
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

use race_solana_types::constants::{
    PROFILE_ACCOUNT_LEN, PROFILE_SEED, PROGRAM_ID, SERVER_ACCOUNT_LEN, SOL,
};
use race_solana_types::instruction::RaceInstruction;
use race_solana_types::types as solana_types;

use serde_json;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;

use solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::message::Message;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::{create_account, create_account_with_seed};
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    vote::state::serde_compact_vote_state_update,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    check_id, check_program_account, id, instruction::initialize_account, state::Account, ID,
};

fn read_keypair(path: PathBuf) -> TransportResult<Keypair> {
    let keypair = solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| TransportError::InvalidKeyfile(e.to_string()))?;
    Ok(keypair)
}

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
            .map_err(|_| TransportError::NoEnoughLamports)?;
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
            .map_err(|_| TransportError::NoEnoughLamports)?;
        let create_temp_account_ix = create_account(
            &payer_pubkey,
            &stake_account_pubkey,
            stake_lamports,
            stake_account_len as u64,
            &ID,
        );

        let init_temp_account_ix =
            initialize_account(&ID, &stake_account_pubkey, &token_pubkey, &payer_pubkey)
                .map_err(|_| TransportError::InitInstructionFailed)?;

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
        // Check endpoint URL len
        if params.endpoint.len() > 50 {
            // FIXME: Use TransportError
            return Err(race_core::error::Error::Custom(
                "Endpoint too long".to_string(),
            ));
        }
        // Create server profile on chain (like creation of a player profile)
        let program_id = Pubkey::from_str(PROGRAM_ID).map_err(|_| TransportError::InvalidConfig)?;
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PROFILE_SEED, &program_id)
                .map_err(|_| TransportError::PubkeyCreationFailed)?;

        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(SERVER_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)?;

        match self.client.get_account(&server_account_pubkey) {
            Ok(_) => {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Server already exists".to_string(),
                ));
            }
            Err(_) => {}
        }

        let create_server_account_ix = create_account_with_seed(
            &payer_pubkey,
            &server_account_pubkey,
            &payer_pubkey,
            PROFILE_SEED,
            lamports,
            SERVER_ACCOUNT_LEN as u64,
            &program_id,
        );

        let ix_data = RaceInstruction::pack(RaceInstruction::RegisterServer {
            params: solana_types::RegisterServerParams {
                endpoint: params.endpoint,
            },
        })
        .map_err(|_| TransportError::InstructionDataError)?;

        let init_account_ix = Instruction::new_with_bytes(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(server_account_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_server_account_ix, init_account_ix],
            Some(&payer_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;

        tx.sign(&[payer], blockhash);

        self.client
            .send_and_confirm_transaction(&tx)
            .map_err(|_| TransportError::ClientSendTransactionFailed)?;

        Ok(server_account_pubkey.to_string())
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

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String> {
        // Check if nick name exceeds the max len (16 chars)
        if params.nick.len() > 16 {
            // FIXME: Transfer transport error to race error
            return Err(race_core::error::Error::Custom(
                "Nick name length should not exceed 16 characters!".to_string(),
            ));
        }
        let program_id = Pubkey::from_str(PROGRAM_ID).map_err(|_| TransportError::InvalidConfig)?;
        let player = &self.keypair;
        let player_pubkey = player.pubkey();
        let balance = self
            .client
            .get_balance(&player_pubkey)
            .map_err(|_| TransportError::InvalidBalance(player_pubkey.to_string()))?;

        let profile_account_pubkey =
            Pubkey::create_with_seed(&player_pubkey, PROFILE_SEED, &program_id)
                .map_err(|_| TransportError::PubkeyCreationFailed)?;

        println!("1");
        let lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(PROFILE_ACCOUNT_LEN)
            .map_err(|_| TransportError::NoEnoughLamports)?;

        // Check if account already exists
        match self.client.get_account(&profile_account_pubkey) {
            Ok(_) => {
                // FIXME: Use TransportError
                return Err(race_core::error::Error::Custom(
                    "Profile already exists".to_string(),
                ));
            }
            Err(_) => {}
        }

        let create_profile_account_ix = create_account_with_seed(
            &player_pubkey,
            &profile_account_pubkey,
            &player_pubkey,
            PROFILE_SEED,
            lamports,
            PROFILE_ACCOUNT_LEN as u64,
            &program_id,
        );

        // TODO: Add Racetoken ATA

        let pfp_pubkey = if params.pfp.is_some() {
            let addr = params.pfp.unwrap();
            Pubkey::from_str(&addr).map_err(|_| TransportError::InvalidPubkey(addr.to_string()))?
        } else {
            let addr = "11111111111111111111111111111111";
            Pubkey::from_str(&addr).map_err(|_| TransportError::InvalidPubkey(addr.to_string()))?
        };

        let ix_data = RaceInstruction::pack(RaceInstruction::CreatePlayerProfile {
            params: solana_types::CreatePlayerProfileParams { nick: params.nick },
        })
        .map_err(|_| TransportError::InstructionDataError)?;

        let init_profile_ix = Instruction::new_with_bytes(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new_readonly(player_pubkey, true),
                AccountMeta::new(profile_account_pubkey, false),
                AccountMeta::new_readonly(pfp_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_profile_account_ix, init_profile_ix],
            Some(&player_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;

        tx.sign(&[player], blockhash);

        self.client
            .send_and_confirm_transaction(&tx)
            .map_err(|_| TransportError::ClientSendTransactionFailed)?;

        Ok(profile_account_pubkey.to_string())
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
        match Pubkey::from_str(addr) {
            Ok(game_account_pubkey) => {
                match self.client.get_account_data(&game_account_pubkey) {
                    Ok(game_account_data) => {
                        match GameState::try_from_slice(&game_account_data) {
                            Ok(state) => {
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
                            Err(e) => {
                                eprintln!("Game State Error {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Game Account Data Error {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Game Account Pubkey Error {}", e);
                None
            }
        }
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        match Pubkey::from_str(addr) {
            Ok(game_account_pubkey) => {
                match &self.client.get_account_data(&game_account_pubkey) {
                    Ok(game_account_data) => {
                        match GameState::try_from_slice(&game_account_data) {
                            Ok(game_state) => {
                                // FIXME: implement GameBundle as NFT and use its metadata
                                let addr = game_state.bundle_addr.to_string();
                                let data = "ARWEAVE BASE64 ADDRESS".to_string();

                                Some(GameBundle {
                                    addr,
                                    data,
                                })
                            }
                            Err(e) => {
                                eprintln!("Game state error: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Game account data error {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Game account pubkey error {}", e);
                None
            }
        }
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        match Pubkey::from_str(addr) {
            Ok(player_pubkey) => {
                match self.client.get_account_data(&player_pubkey) {
                    Ok(profile_account_data) => {
                        println!("Account data to deserialize {:?}", profile_account_data);
                        match PlayerState::try_from_slice(&profile_account_data) {
                            Ok(player_state) => {
                                let addr = player_state.addr.to_string();
                                let pfp = if player_state.pfp.is_some() {
                                    Some(player_state.pfp.unwrap().to_string())
                                } else {
                                    None
                                };
                                Some(PlayerProfile {
                                    addr,
                                    nick: player_state.nick,
                                    pfp,
                                })
                            }
                            Err(e) => {
                                eprintln!("Profile account data error: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Profile account error: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Player profile pubkey error {}", e);
                None
            }
        }
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        match Pubkey::from_str(addr) {
            Ok(server_account_pubkey) => {
                match self.client.get_account_data(&server_account_pubkey) {
                    Ok(server_account_data) => {
                        println!("Account data to deserialize {:?}", server_account_data);
                        match ServerState::try_from_slice(&server_account_data) {
                            Ok(server_state) => Some(ServerAccount {
                                addr: server_state.addr.to_string(),
                                owner_addr: server_state.owner.to_string(),
                                endpoint: server_state.endpoint,
                            }),
                            Err(e) => {
                                eprintln!("Server account data error: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Server account error: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Server pubkey error {}", e);
                None
            }
        }
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
    use solana_client::rpc_config::RpcProgramAccountsConfig;

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

        println!("To close game account {}", addr);
        transport
            .close_game_account(CloseGameAccountParams { addr })
            .await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_player_profile() -> anyhow::Result<()> {
        let transport = get_transport()?;
        // Create a player profile
        let profile_addr = transport.create_player_profile(CreatePlayerProfileParams {
            addr: "HHHHHJJJJKKKKLLLLPPPPOOOOIIIIUUUU".to_string(),
            nick: "Jackson".to_owned(),
            pfp: None
        })
            .await?;

        println!("Created profile is {}", profile_addr);

        // Try to get it
        // let profile_addr: &str = "FEZ6ki7Jy1fG4sYLwEiDZQm1bk5H7v4JpLc3EHKs355K";
        let profile = transport.get_player_profile(&profile_addr).await.unwrap();
        assert_eq!("Jackson".to_string(), profile.nick);
        assert_eq!(None, profile.pfp);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_server_account() -> anyhow::Result<()> {
        let transport = get_transport()?;
        // Create a player profile
        let server_addr = transport
            .register_server(RegisterServerParams {
                endpoint: "https://api.testnet.solana.com".to_string(),
            })
            .await?;

        println!("Created profile is {}", server_addr);

        // Try to get it
        // let addr = "8BUgJXM54YbiLSFf9pYjUejSej39G8U9VeAimQdmd43u";
        let server_state = transport.get_server_account(&server_addr).await.unwrap();
        assert_eq!(
            "https://api.testnet.solana.com".to_string(),
            server_state.endpoint
        );

        Ok(())
    }
}
