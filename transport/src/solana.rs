#![cfg(not(target_arch = "wasm32"))]
#![allow(unused_variables, unused_imports)]
use crate::error::{TransportError, TransportResult};
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use jsonrpsee::core::Error;
use mpl_token_metadata::{processor::mint_v1, utils::spl_token_mint_to};
use race_core::{
    error::Result,
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration,
        JoinParams, PlayerJoin, PlayerProfile, PublishGameParams, RegisterGameParams,
        RegisterServerParams, RegistrationAccount, ServeParams, ServerAccount, ServerJoin,
        SettleOp, SettleParams, UnregisterGameParams, VoteParams,
    },
};
use race_solana_types::constants::{
    GAME_ACCOUNT_LEN, MAX_SERVER_NUM, NAME_LEN, NATIVE_MINT, PLAYER_PROFILE_SEED,
    PROFILE_ACCOUNT_LEN, PROGRAM_ID, RACE_ATA, RACE_MINT, REGISTRY_ACCOUNT_LEN, SERVER_ACCOUNT_LEN,
    SERVER_PROFILE_SEED,
};
use race_solana_types::instruction::RaceInstruction;
use race_solana_types::state::{self, GameReg, GameState, PlayerState, RegistryState, ServerState};
use race_solana_types::types as solana_types;

use serde_json;
// use core::slice::SlicePattern;
use std::path::PathBuf;
use std::str::FromStr;
use std::{
    borrow::BorrowMut,
    fs::{read_to_string, File},
};

use mpl_token_metadata as metaplex_program;
use mpl_token_metadata::pda::find_metadata_account;
use mpl_token_metadata::state::Metadata;
use solana_client::{
    rpc_client::{RpcClient, RpcClientConfig},
    rpc_config::RpcSendTransactionConfig,
};
use solana_sdk::system_instruction::{self, create_account, create_account_with_seed, transfer};
use solana_sdk::transaction::Transaction;
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack};
use solana_sdk::{feature_set::separate_nonce_from_blockhash, pubkey::Pubkey};
use solana_sdk::{feature_set::zk_token_sdk_enabled, signature::Keypair};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::Message,
    program_error::ProgramError,
    pubkey::ParsePubkeyError,
    signature::Signature,
    system_program,
    sysvar::{self, rent},
};
use solana_sdk::{signer::Signer, stake::instruction::create_account_and_delegate_stake};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{
    instruction as spl_token_instruction,
    instruction::{initialize_account, sync_native},
    state::{Account, Mint},
};

fn read_keypair(path: PathBuf) -> TransportResult<Keypair> {
    let keypair = solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| TransportError::InvalidKeyfile(e.to_string()))?;
    Ok(keypair)
}

pub struct SolanaTransport {
    program_id: Pubkey,
    client: RpcClient,
    keypair: Keypair,
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        // TODO: Discuss title allowed len
        if params.title.len() > NAME_LEN {
            // FIXME: Use TransportError
            return Err(TransportError::InvalidNameLength(params.title))?;
        }

        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let bundle_pubkey = Self::parse_pubkey(&params.bundle_addr)?;
        let game_account = Keypair::new();
        let game_account_pubkey = game_account.pubkey();
        let lamports = self.get_min_lamports(GAME_ACCOUNT_LEN)?;
        let create_game_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &game_account_pubkey,
            lamports,
            GAME_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        // TODO: Use RACE ATA?
        // Create stake account to hold players deposits
        let token_mint_pubkey = Self::parse_pubkey(&params.token_addr)?;
        let stake_account = Keypair::new();
        let stake_account_pubkey = stake_account.pubkey();
        let stake_account_len = Account::LEN;
        let stake_lamports = self.get_min_lamports(stake_account_len)?;
        let create_stake_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &stake_account_pubkey,
            stake_lamports,
            stake_account_len as u64,
            &spl_token::id(),
        );

        let init_stake_account_ix = spl_token_instruction::initialize_account(
            &spl_token::id(),
            &stake_account_pubkey,
            &token_mint_pubkey,
            &payer_pubkey,
        )
        .map_err(|e| TransportError::InstructionCreationError(e.to_string()))?;

        let create_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreateGameAccount {
                params: race_solana_types::types::CreateGameAccountParams {
                    title: params.title,
                    max_players: params.max_players,
                    min_deposit: params.min_deposit,
                    max_deposit: params.max_deposit,
                    data: params.data,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, true),
                AccountMeta::new_readonly(token_mint_pubkey, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(bundle_pubkey, false),
            ],
        );

        let message = Message::new(
            &[
                create_game_account_ix,
                create_stake_account_ix,
                init_stake_account_ix,
                create_game_ix,
            ],
            Some(&payer.pubkey()),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer, &game_account, &stake_account], blockhash);
        self.send_transaction(tx)?;
        Ok(game_account_pubkey.to_string())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        // payer is initializer/owner of the to-be-closed game
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let game_account_pubkey = Self::parse_pubkey(&params.addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;
        let stake_account_pubkey = game_state.stake_account.clone();

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        let close_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CloseGameAccount,
            vec![
                AccountMeta::new(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new_readonly(pda, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        );

        let message = Message::new(&[close_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        // Check endpoint URL len
        if params.endpoint.len() > 50 {
            return Err(TransportError::EndpointTooLong)?;
        }
        // Create server profile on chain (like creation of a player profile)
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;
        let lamports = self.get_min_lamports(SERVER_ACCOUNT_LEN)?;

        // match self.client.get_account(&server_account_pubkey) {
        //     Ok(_) => {
        //         return Err(TransportError::DuplicateServerAccount)?;
        //     }
        //     _ => {}
        // }

        // Get a server account; same seed leads to same result
        let get_server_account_ix = create_account_with_seed(
            &payer_pubkey,
            &server_account_pubkey,
            &payer_pubkey,
            SERVER_PROFILE_SEED,
            lamports,
            SERVER_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        // Init a new account or update the old endpoint
        let init_or_update_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::RegisterServer {
                params: solana_types::RegisterServerParams {
                    endpoint: params.endpoint,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(server_account_pubkey, false),
            ],
        );

        let message = Message::new(
            &[get_server_account_ix, init_or_update_ix],
            Some(&payer_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);

        self.send_transaction(tx)?;
        Ok(server_account_pubkey.to_string())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let player_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;

        if game_state.access_version != params.access_version {
            return Err(TransportError::AccessVersionNotMatched)?;
        }

        let mint_pubkey = game_state.token_mint.clone();
        let payer_ata = get_associated_token_address(&payer_pubkey, &mint_pubkey);

        let is_wsol = mint_pubkey == spl_token::native_mint::id();

        let stake_account_pubkey = game_state.stake_account.clone();

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[game_account_pubkey.as_ref()], &self.program_id);

        let mut ixs = vec![];

        let temp_account = Keypair::new();
        let temp_account_pubkey = temp_account.pubkey();
        let temp_account_len = Account::LEN;
        let temp_account_lamports = self.get_min_lamports(temp_account_len)?;
        let create_temp_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &temp_account_pubkey,
            temp_account_lamports,
            temp_account_len as u64,
            &spl_token::id(),
        );
        ixs.push(create_temp_account_ix);

        let init_temp_account_ix = initialize_account(
            &spl_token::id(),
            &temp_account_pubkey,
            &mint_pubkey,
            &payer_pubkey,
        )
        .map_err(|_| TransportError::InitInstructionFailed)?;
        ixs.push(init_temp_account_ix);

        if is_wsol {
            let amount = params.amount - temp_account_lamports;
            let transfer_ix = transfer(&payer_pubkey, &temp_account_pubkey, amount);
            let sync_ix = sync_native(&spl_token::id(), &temp_account_pubkey)
                .map_err(|e| TransportError::InstructionCreationError(e.to_string()))?;

            ixs.push(transfer_ix);
            ixs.push(sync_ix);
        } else {
            let spl_transfer_ix = spl_token::instruction::transfer(
                &spl_token::id(),
                &payer_ata,
                &temp_account_pubkey,
                &payer_pubkey,
                &[&payer_pubkey],
                params.amount,
            )
            .map_err(|e| TransportError::InstructionCreationError(e.to_string()))?;
            ixs.push(spl_transfer_ix);
        }

        let join_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::JoinGame {
                params: race_solana_types::types::JoinParams {
                    amount: params.amount,
                    access_version: params.access_version,
                    position: params.position,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new_readonly(player_account_pubkey, false),
                AccountMeta::new(temp_account_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new(pda, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        );
        ixs.push(join_game_ix);

        let message = Message::new(&ixs, Some(&payer_pubkey));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer, &temp_account], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let serve_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::ServeGame,
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new_readonly(server_account_pubkey, false),
            ],
        );

        let message = Message::new(&[serve_game_ix], Some(&payer_pubkey));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String> {
        // Check if nick name exceeds 16 chars
        if params.nick.len() > NAME_LEN {
            return Err(TransportError::InvalidNameLength(params.nick))?;
        }

        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        let profile_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        println!(
            "Profile account pubkey: {}",
            profile_account_pubkey.to_string()
        );
        let mut ixs = Vec::new();

        // Check if player account already exists
        if self.client.get_account(&profile_account_pubkey).is_err() {
            let lamports = self.get_min_lamports(PROFILE_ACCOUNT_LEN)?;
            let create_profile_account_ix = create_account_with_seed(
                &payer_pubkey,
                &profile_account_pubkey,
                &payer_pubkey,
                PLAYER_PROFILE_SEED,
                lamports,
                PROFILE_ACCOUNT_LEN as u64,
                &self.program_id,
            );
            ixs.push(create_profile_account_ix);
        }

        let pfp_pubkey = if params.pfp.is_some() {
            let addr = params.pfp.unwrap();
            Self::parse_pubkey(&addr)?
        } else {
            system_program::id()
        };

        let init_profile_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreatePlayerProfile {
                params: solana_types::CreatePlayerProfileParams { nick: params.nick },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(profile_account_pubkey, false),
                AccountMeta::new_readonly(pfp_pubkey, false),
            ],
        );

        ixs.push(init_profile_ix);

        let message = Message::new(&ixs, Some(&payer_pubkey));

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(profile_account_pubkey.to_string())
    }

    // TODO: add close_player_profile

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();

        // Create token and initialize its mint
        let new_mint = Keypair::new();
        let mint_pubkey = new_mint.pubkey();
        let mint_account_lamports = self.get_min_lamports(Mint::LEN)?;
        let create_mint_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &mint_pubkey,
            mint_account_lamports,
            Mint::LEN as u64,
            &spl_token::id(),
        );

        // Set decimals to 0 (NFT)
        let init_mint_ix = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_pubkey,
            &payer_pubkey,
            Some(&payer_pubkey),
            0,
        )
        .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;

        // Generate two PDAs from mint_account:
        // one for metadata account and the other for master edition account
        let (metadata_pda, bump_seed) = Pubkey::find_program_address(
            &[
                "metadata".as_bytes(),
                metaplex_program::id().as_ref(),
                mint_pubkey.as_ref(),
            ],
            &metaplex_program::id(),
        );

        let (edition_pda, bump_seed) = Pubkey::find_program_address(
            &[
                "metadata".as_bytes(),
                metaplex_program::id().as_ref(),
                mint_pubkey.as_ref(),
                "edition".as_bytes(),
            ],
            &metaplex_program::id(),
        );

        // Create ATA to hold the new mint
        let ata_pubkey = get_associated_token_address(&payer_pubkey, &mint_pubkey);
        let create_ata_account_ix = create_associated_token_account(
            &payer_pubkey,
            &payer_pubkey,
            &mint_pubkey,
            &spl_token::id(),
        );

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(mint_pubkey, true),
            AccountMeta::new_readonly(ata_pubkey, false),
            AccountMeta::new(metadata_pda, false),
            AccountMeta::new(edition_pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(metaplex_program::id(), false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let publish_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::PublishGame {
                params: race_solana_types::types::PublishParams {
                    uri: params.uri,
                    name: params.name,
                    symbol: params.symbol,
                },
            },
            accounts,
        );

        let message = Message::new(
            &[
                create_mint_account_ix,
                init_mint_ix,
                // create_token_account_ix,
                // init_token_account_ix,
                // mint_token_ix,
                create_ata_account_ix,
                publish_game_ix,
            ],
            Some(&payer_pubkey),
        );

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer, &new_mint], blockhash);
        self.send_transaction(tx)?;

        Ok(mint_pubkey.to_string())
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        let SettleParams { addr, mut settles } = params;
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;
        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        // The settles are required to be in correct order: add < sub < eject.
        // And the sum of settles must be zero.
        settles.sort_by_key(|s| match s.op {
            SettleOp::Eject => 2,
            SettleOp::Add(_) => 0,
            SettleOp::Sub(_) => 1,
        });

        let settles: TransportResult<Vec<race_solana_types::types::Settle>> = settles
            .into_iter()
            .map(|s| {
                Ok(match s.op {
                    SettleOp::Eject => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Eject,
                    },
                    SettleOp::Add(amt) => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Add(amt),
                    },
                    SettleOp::Sub(amt) => race_solana_types::types::Settle {
                        addr: Pubkey::from_str(&s.addr)?,
                        op: race_solana_types::types::SettleOp::Sub(amt),
                    },
                })
            })
            .collect();
        let settles = settles?;

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(Pubkey::from_str(&addr).unwrap(), false),
            AccountMeta::new_readonly(game_state.token_mint.clone(), false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let settle_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::Settle {
                params: race_solana_types::types::SettleParams { settles },
            },
            accounts,
        );

        let message = Message::new(&[settle_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let registry_account = Keypair::new();
        let registry_account_pubkey = registry_account.pubkey();
        let lamports = self.get_min_lamports(REGISTRY_ACCOUNT_LEN)?;
        let create_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &registry_account_pubkey,
            lamports,
            REGISTRY_ACCOUNT_LEN as u64,
            &self.program_id,
        );
        let create_registry_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::CreateRegistry {
                params: race_solana_types::types::CreateRegistrationParams {
                    is_private: params.is_private,
                    size: params.size,
                },
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(registry_account_pubkey, false),
            ],
        );

        let message = Message::new(
            &[create_account_ix, create_registry_ix],
            Some(&payer.pubkey()),
        );
        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&payer, &registry_account], blockhash);
        self.send_transaction(tx)?;
        let addr = registry_account_pubkey.to_string();
        Ok(addr)
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        let reg_state = self.internal_get_registry_state(&reg_account_pubkey).await?;
        // println!("payer pubkey {:?}", payer_pubkey);
        // println!("game pubkey {:?}", game_account_pubkey);
        // println!("reg pubkey {:?}", reg_account_pubkey);
        // println!("reg_state owner {:?}", reg_state.owner);

        if reg_state.games.len() == reg_state.size as usize {
            // FIXME: Use TransportError
            return Err(race_core::error::Error::Custom(
                "Registry already full".to_string(),
            ));
        }

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(reg_account_pubkey, false),
            AccountMeta::new_readonly(game_account_pubkey, false),
        ];

        let register_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            &RaceInstruction::RegisterGame, // TODO: add is_hidden
            accounts,
        );

        let message = Message::new(&[register_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        let payer = &self.keypair;
        let payer_pubkey = payer.pubkey();
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        // let reg_state = self.get_registry_state(&reg_account_pubkey).await?;

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(reg_account_pubkey, false),
            AccountMeta::new_readonly(game_account_pubkey, false),
        ];

        let unregister_game_ix = Instruction::new_with_borsh(
            self.program_id.clone(),
            // TODO: add is_hidden param?
            &RaceInstruction::UnregisterGame,
            accounts,
        );

        let message = Message::new(&[unregister_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let game_account_pubkey = Self::parse_pubkey(addr).ok()?;
        let state: GameState = self
            .internal_get_game_state(&game_account_pubkey)
            .await
            .ok()?;
        let transactor_addr = match state.transactor_addr {
            Some(pubkey) => Some(pubkey.to_string()),
            None => None,
        };

        Some(GameAccount {
            addr: addr.to_owned(),
            title: state.title,
            settle_version: state.settle_version,
            bundle_addr: state.bundle_addr.to_string(),
            token_addr: state.token_mint.to_string(),
            owner_addr: state.owner.to_string(),
            access_version: state.access_version,
            min_deposit: state.min_deposit,
            max_deposit: state.max_deposit,
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
            deposits: Vec::new(),
            votes: Vec::new(),
            unlock_time: None,
        })
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        let mint_pubkey = Self::parse_pubkey(addr).ok()?;

        let (metadata_account_pubkey, _) =
            metaplex_program::pda::find_metadata_account(&mint_pubkey);
        // println!(
        //     "Metadata account (PDA of MINT) {:?}",
        //     metadata_account_pubkey
        // );

        let metadata_account_data = self
            .client
            .get_account_data(&metadata_account_pubkey)
            .ok()?;
        let metadata_account_state =
            Metadata::deserialize(&mut metadata_account_data.as_slice()).map_err(|_| TransportError::MetadataDeserializeError).ok()?;
        let metadata_data = metadata_account_state.data;
        let uri = metadata_data.uri.trim_end_matches('\0').to_string();

        let data = race_nft_storage::fetch_wasm_from_game_bundle(&uri)
            .await
            .ok()?;

        Some(GameBundle {
            uri,
            name: metadata_data.name.trim_end_matches('\0').to_string(),
            data,
        })
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        let wallet_pubkey = Self::parse_pubkey(addr).ok()?;
        let profile_state = self.internal_get_player_state(&wallet_pubkey).await.ok()?;
        let pfp = profile_state.pfp.map(|x| x.to_string());
        Some(PlayerProfile {
            addr: addr.to_owned(),
            nick: profile_state.nick,
            pfp,
        })
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let wallet_pubkey = Self::parse_pubkey(addr).ok()?;
        let server_state = self.internal_get_server_state(&wallet_pubkey).await.ok()?;

        Some(ServerAccount {
            addr: server_state.owner.to_string(),
            endpoint: server_state.endpoint,
        })
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let key = Self::parse_pubkey(addr).ok()?;
        let state = self.internal_get_registry_state(&key).await.ok()?;

        Some(RegistrationAccount {
            addr: addr.to_owned(),
            is_private: state.is_private,
            size: state.size,
            owner: Some(state.owner.to_string()),
            games: state
                .games
                .into_iter()
                .map(|g| GameRegistration {
                    title: g.title.clone(),
                    addr: g.addr.to_string(),
                    reg_time: g.reg_time,
                    bundle_addr: "".into(),
                })
                .collect(),
        })
    }
}

impl SolanaTransport {
    pub fn try_new(rpc: String, keyfile: PathBuf) -> TransportResult<Self> {
        let program_id = Pubkey::from_str(PROGRAM_ID)?;
        SolanaTransport::try_new_with_program_id(rpc, keyfile, program_id)
    }

    #[allow(unused)]
    pub(crate) fn wallet_pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    pub(crate) fn try_new_with_program_id(
        rpc: String,
        keyfile: PathBuf,
        program_id: Pubkey,
    ) -> TransportResult<Self> {
        println!(
            "Create Solana transport: RPC: {}, program_id: {:?}",
            rpc, program_id
        );
        let commitment = if cfg!(test) {
            CommitmentConfig::confirmed()
        } else {
            CommitmentConfig::finalized()
        };
        let client = RpcClient::new_with_commitment(rpc, commitment);
        let keypair = read_keypair(keyfile)?;
        Ok(Self {
            client,
            keypair,
            program_id,
        })
    }

    fn parse_pubkey(addr: &str) -> TransportResult<Pubkey> {
        Pubkey::from_str(addr).map_err(|_| TransportError::InvalidConfig)
    }

    fn get_min_lamports(&self, account_len: usize) -> TransportResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(account_len)
            .map_err(|_| TransportError::NoEnoughLamports)
    }

    fn get_blockhash(&self) -> TransportResult<Hash> {
        self.client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)
    }

    fn send_transaction(&self, tx: Transaction) -> TransportResult<Signature> {
        // let sig = self
        //     .client
        //     .send_and_confirm_transaction(&tx)
        //     .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        // let skip_preflight = if cfg!(test) { true } else { false };
        let skip_preflight = true;
        let confirm_num = if cfg!(test) { 1 } else { 32 };

        let sig = self
            .client
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight,
                    ..RpcSendTransactionConfig::default()
                },
            )
            .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        self.client
            .poll_for_signature_confirmation(&sig, confirm_num)
            .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        Ok(sig)
    }

    /// Get the state of an on-chain game account by its public key.
    /// Not for public API usage
    async fn internal_get_game_state(
        &self,
        game_account_pubkey: &Pubkey,
    ) -> TransportResult<GameState> {
        let data = self
            .client
            .get_account_data(&game_account_pubkey)
            .or(Err(TransportError::GameAccountNotFound))?;
        GameState::deserialize(&mut data.as_slice())
            .map_err(|_| TransportError::GameStateDeserializeError)
    }

    /// Get the state of an on-chain server account
    /// Not for public API usage
    #[allow(dead_code)]
    async fn internal_get_server_state(&self, server_pubkey: &Pubkey) -> TransportResult<ServerState> {
        let server_account_pubkey =
            Pubkey::create_with_seed(server_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let data = self
            .client
            .get_account_data(&server_account_pubkey)
            .or(Err(TransportError::ServerAccountDataNotFound))?;
        ServerState::deserialize(&mut data.as_slice())
            .map_err(|e| TransportError::ServerStateDeserializeError)
    }

    async fn internal_get_player_state(&self, player_pubkey: &Pubkey) -> TransportResult<PlayerState> {
        let profile_pubkey =
            Pubkey::create_with_seed(&player_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let data = self.client.get_account_data(&profile_pubkey)
            .or(Err(TransportError::PlayerAccountDataNotFound))?;

        PlayerState::deserialize(&mut data.as_slice())
            .map_err(|_| TransportError::PlayerStateDeserializeError)
    }

    /// Get the state of an on-chain registry account
    /// Not for public API usage
    async fn internal_get_registry_state(
        &self,
        registry_account_pubkey: &Pubkey,
    ) -> TransportResult<RegistryState> {
        let data = self
            .client
            .get_account_data(&registry_account_pubkey)
            .or(Err(TransportError::RegistryAccountDataNotFound))?;

        RegistryState::deserialize(&mut data.as_slice())
            .map_err(|e| TransportError::RegistryStateDeserializeError)
    }
}

impl From<ParsePubkeyError> for TransportError {
    fn from(value: ParsePubkeyError) -> Self {
        TransportError::ParseAddressError
    }
}

#[cfg(test)]
mod tests {
    use solana_client::rpc_config::RpcProgramAccountsConfig;

    use super::*;

    fn read_program_id() -> anyhow::Result<Pubkey> {
        let proj_root = project_root::get_project_root()?;
        let keyfile_path = proj_root.join("target/deploy/race_solana-keypair.json".to_string());
        let program_keypair = read_keypair(keyfile_path)?;
        let program_id = program_keypair.pubkey();
        println!("program id: {}", program_id);
        Ok(program_id)
    }

    #[test]
    fn test_read_program_id() -> anyhow::Result<()> {
        read_program_id()?;
        Ok(())
    }

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
        let transport = SolanaTransport::try_new_with_program_id(
            "http://localhost:8899".into(),
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
            read_program_id()?,
        )?;
        Ok(transport)
    }

    #[test]
    fn test_get_transport() -> anyhow::Result<()> {
        get_transport()?;
        Ok(())
    }

    async fn create_player(transport: &SolanaTransport) -> anyhow::Result<String> {
        let player = transport
            .create_player_profile(CreatePlayerProfileParams {
                nick: "Alice".to_string(),
                pfp: None,
            })
            .await?;
        Ok(player)
    }

    async fn create_game(transport: &SolanaTransport) -> anyhow::Result<String> {
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "16-CHAR_GAME_TIL".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                token_addr: RACE_MINT.to_string(),
                min_deposit: 10,
                max_deposit: 100,
                max_players: 9,
                data: Vec::<u8>::new(),
            })
            .await?;
        println!("Create game at {}", addr);
        Ok(addr)
    }

    async fn create_reg(transport: &SolanaTransport) -> anyhow::Result<String> {
        let transport = get_transport()?;
        let addr = transport
            .create_registration(CreateRegistrationParams {
                is_private: false,
                size: 100,
            })
            .await?;
        Ok(addr)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_game_create_get_close() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = create_game(&transport).await?;
        let game_account = transport
            .get_game_account(&addr)
            .await
            .expect("Failed to query");
        assert_eq!(game_account.access_version, 0);
        assert_eq!(game_account.settle_version, 0);
        assert_eq!(game_account.max_players, 9);
        assert_eq!(game_account.title, "16-CHAR_GAME_TIL");
        transport
            .close_game_account(CloseGameAccountParams { addr: addr.clone() })
            .await
            .expect("Failed to close");
        assert_eq!(None, transport.get_game_account(&addr).await);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_registry_create_get() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = create_reg(&transport).await?;
        let reg = transport.get_registration(&addr).await.unwrap();
        assert_eq!(reg.is_private, false);
        assert_eq!(reg.size, 100);
        assert_eq!(reg.games.len(), 0);
        let game_addr = create_game(&transport).await?;
        transport
            .register_game(RegisterGameParams {
                game_addr,
                reg_addr: addr.clone(),
            })
            .await?;
        let reg = transport.get_registration(&addr).await.unwrap();
        assert_eq!(reg.games.len(), 1);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_server() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let endpoint = "https://api.testnet.solana.com".to_string();
        let addr = transport
            .register_server(RegisterServerParams {
                endpoint: endpoint.clone(),
            })
            .await?;

        let server = transport
            .get_server_account(&transport.wallet_pubkey().to_string())
            .await
            .unwrap();
        assert_eq!(server.addr, transport.wallet_pubkey().to_string());
        assert_eq!(server.endpoint, endpoint);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_player_profile() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let nick = "Foo".to_string();
        transport
            .create_player_profile(CreatePlayerProfileParams {
                nick: nick.clone(),
                pfp: None,
            })
            .await?;
        let profile = transport
            .get_player_profile(&transport.wallet_pubkey().to_string())
            .await
            .unwrap();
        assert_eq!(profile.addr, transport.wallet_pubkey().to_string());
        assert_eq!(profile.nick, nick);
        assert_eq!(profile.pfp, None);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_serve_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let game_addr = create_game(&transport).await?;
        let server_addr = transport
            .serve(ServeParams {
                game_addr: game_addr.clone(),
            })
            .await?;
        let game = transport
            .get_game_account(&game_addr)
            .await
            .expect("Failed to get game");
        assert_eq!(game.servers.len(), 1);
        assert_eq!(
            game.transactor_addr,
            Some(transport.wallet_pubkey().to_string())
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_join() -> anyhow::Result<()> {
        let transport = get_transport()?;
        create_player(&transport).await?;
        let game_addr = create_game(&transport).await?;
        println!("Join game: {}", game_addr);
        transport
            .join(JoinParams {
                game_addr: game_addr.clone(),
                amount: 50u64,
                access_version: 0u64,
                position: 0usize,
            })
            .await?;
        let game = transport
            .get_game_account(&game_addr)
            .await
            .expect("Failed to get game");
        assert_eq!(game.players.len(), 1);
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_publish_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let params = PublishGameParams {
            uri: "https://arweave.net/uQFXQ9Jp5IrO5qGuTX8zSWRMJU679M6ZGW9MM1cSP0E".to_string(),
            name: "RACE_raffle".to_string(),
            symbol: "RACE".to_string(),
        };
        let token_mint = transport.publish_game(params).await?;
        println!("Publish game mint {}", token_mint);

        let bundle = transport
            .get_game_bundle(&token_mint)
            .await
            .expect("Failed to get game bundle");

        assert_eq!(bundle.name, "RACE_raffle".to_string());
        assert_eq!(
            bundle.uri,
            "https://arweave.net/uQFXQ9Jp5IrO5qGuTX8zSWRMJU679M6ZGW9MM1cSP0E".to_string()
        );
        Ok(())
    }

    #[allow(dead_code)]
    async fn test_settle() -> anyhow::Result<()> {
        // let game_addr = create_game();
        Ok(())
    }
}
