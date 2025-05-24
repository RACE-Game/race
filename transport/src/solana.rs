mod constants;
mod metadata;
mod types;

use async_stream::stream;
use constants::*;
use futures::{Stream, StreamExt};
use solana_transaction_status::UiTransactionEncoding;
use std::time::Duration;
use tracing::{error, info, warn};
use types::*;

use crate::error::{TransportError, TransportResult};
use async_trait::async_trait;
use borsh::BorshDeserialize;
use race_core::{
    error::{Error, Result},
    transport::TransportT,
    types::{
        AddRecipientSlotParams, AssignRecipientParams, Award, CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, GameRegistration, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegisterServerParams, RegistrationAccount, RejectDepositsParams, RejectDepositsResult, ServeParams, ServerAccount, SettleParams, SettleResult, UnregisterGameParams, VoteParams
    },
};

use std::str::FromStr;
use std::{path::PathBuf, pin::Pin};

use solana_account_decoder::UiAccountEncoding;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcAccountInfoConfig, RpcSendTransactionConfig};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::{self, create_account_with_seed, transfer};
use solana_sdk::transaction::Transaction;
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::ParsePubkeyError,
    signature::Signature,
    system_program,
    sysvar::rent,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{
    instruction as spl_token_instruction,
    instruction::{initialize_account, sync_native},
    state::{Account, Mint},
};

mod nft;

const METAPLEX_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

fn read_keypair(path: PathBuf) -> TransportResult<Keypair> {
    let keypair = solana_sdk::signature::read_keypair_file(path)
        .map_err(|e| TransportError::InvalidKeyfile(e.to_string()))?;
    Ok(keypair)
}

fn is_native_mint(mint_pubkey: &Pubkey) -> bool {
    mint_pubkey.to_string().eq(NATIVE_MINT)
}

pub struct SolanaTransport {
    rpc: String,
    program_id: Pubkey,
    client: RpcClient,
    keypair: Option<Keypair>,
    debug: bool,
}

#[async_trait]
impl TransportT for SolanaTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        // TODO: Discuss title allowed len
        if params.title.len() > NAME_LEN {
            return Err(TransportError::InvalidNameLength(params.title))?;
        }

        // Save all used keys for later fee calculation
        let mut used_keys = vec![];

        let (payer, payer_pubkey) = self.payer()?;
        let bundle_pubkey = Self::parse_pubkey(&params.bundle_addr)?;
        used_keys.push(payer_pubkey.clone());

        let game_account = Keypair::new();
        let game_account_pubkey = game_account.pubkey();
        let lamports = self.get_min_lamports(GAME_ACCOUNT_LEN)?;
        used_keys.push(game_account_pubkey.clone());

        let mut ixs = vec![];
        let create_game_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &game_account_pubkey,
            lamports,
            GAME_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        ixs.push(create_game_account_ix);
        let recipient_pubkey = Self::parse_pubkey(&params.recipient_addr)?;
        used_keys.push(recipient_pubkey.clone());
        let token_mint_pubkey = Self::parse_pubkey(&params.token_addr)?;

        let (stake_account_pubkey, stake_account) = if is_native_mint(&token_mint_pubkey) {
            // For game with SOL, use PDA as stake account

            let (pda, _bump_seed) =
                Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

            (pda, None)
        } else {
            // For game with SPL, use dedicated stake account
            let stake_account_len = Account::LEN;
            let stake_lamports = self.get_min_lamports(stake_account_len)?;
            let stake_account = Keypair::new();
            let stake_account_pubkey = stake_account.pubkey();
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

            used_keys.push(stake_account_pubkey.clone());

            ixs.push(create_stake_account_ix);
            ixs.push(init_stake_account_ix);

            (stake_account_pubkey, Some(stake_account))
        };

        let create_game_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::CreateGameAccount {
                params: params.into(),
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new_readonly(token_mint_pubkey, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(bundle_pubkey, false),
                AccountMeta::new_readonly(recipient_pubkey, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        ixs.push(create_game_ix);

        let fee =
            self.get_recent_prioritization_fees(&[game_account_pubkey, stake_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);
        ixs.insert(0, set_cu_prize_ix);

        let message = Message::new(&ixs, Some(&payer.pubkey()));

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;

        if let Some(stake_account) = stake_account {
            tx.sign(&[payer, &game_account, &stake_account], blockhash);
        } else {
            tx.sign(&[payer, &game_account], blockhash);
        }

        self.send_transaction(tx)?;
        Ok(game_account_pubkey.to_string())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;

        let game_account_pubkey = Self::parse_pubkey(&params.addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;
        let stake_account_pubkey = game_state.stake_account;

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        // For game with SOL, PDA = stake account
        let receiver_pubkey = if stake_account_pubkey.eq(&pda) {
            info!("Game uses SOL, receiver account: {}", payer_pubkey);
            payer_pubkey
        } else {
            let ata_account_pubkey =
                get_associated_token_address(&payer_pubkey, &game_state.token_mint);
            info!("Game uses SPL, receiver account: {}", ata_account_pubkey);
            ata_account_pubkey
        };

        let mut keys = vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new(stake_account_pubkey, false),
                AccountMeta::new_readonly(pda, false),
                AccountMeta::new(receiver_pubkey, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
        ];

        // We need bonus and ATAs for all bonuses
        for bonus in game_state.bonuses.iter() {
            let bonus_ata_pubkey =
                get_associated_token_address(&payer_pubkey, &bonus.token_addr);
            keys.push(AccountMeta::new(bonus.stake_addr, false));
            keys.push(AccountMeta::new(bonus_ata_pubkey, false));
        }

        let close_game_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::CloseGameAccount,
            keys,
        );

        let fee =
            self.get_recent_prioritization_fees(&[game_account_pubkey, stake_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(&[set_cu_prize_ix, close_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        if params.endpoint.len() > 50 {
            return Err(TransportError::EndpointTooLong)?;
        }
        let (payer, payer_pubkey) = self.payer()?;

        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;
        let lamports = self.get_min_lamports(SERVER_ACCOUNT_LEN)?;

        let mut ixs = vec![];

        let fee = self.get_recent_prioritization_fees(&[server_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        ixs.push(set_cu_prize_ix);

        if self
            .client
            .get_account_data(&server_account_pubkey)
            .is_err()
        {
            let create_server_account_ix = create_account_with_seed(
                &payer_pubkey,
                &server_account_pubkey,
                &payer_pubkey,
                SERVER_PROFILE_SEED,
                lamports,
                SERVER_ACCOUNT_LEN as u64,
                &self.program_id,
            );
            ixs.push(create_server_account_ix);
        }

        let init_or_update_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::RegisterServer {
                params: params.into(),
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(server_account_pubkey, false),
            ],
        );

        ixs.push(init_or_update_ix);

        let message = Message::new(&ixs, Some(&payer_pubkey));

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);

        self.send_transaction(tx)?;
        Ok(())
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;

        let player_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;

        let mint_pubkey = game_state.token_mint;
        let payer_ata = get_associated_token_address(&payer_pubkey, &mint_pubkey);

        let is_wsol = mint_pubkey == spl_token::native_mint::id();

        let stake_account_pubkey = game_state.stake_account;

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
            self.program_id,
            &RaceInstruction::JoinGame {
                params: params.into(),
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
                AccountMeta::new_readonly(system_program::id(), false),
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

    async fn deposit(&self, _params: DepositParams) -> Result<()> {
        todo!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;

        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let server_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let serve_game_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::ServeGame {
                params: params.into(),
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(game_account_pubkey, false),
                AccountMeta::new_readonly(server_account_pubkey, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let fee = self.get_recent_prioritization_fees(&[game_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(&[set_cu_prize_ix, serve_game_ix], Some(&payer_pubkey));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn vote(&self, _params: VoteParams) -> Result<()> {
        todo!()
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        if params.nick.len() > NAME_LEN {
            return Err(TransportError::InvalidNameLength(params.nick))?;
        }

        let (payer, payer_pubkey) = self.payer()?;

        let profile_account_pubkey =
            Pubkey::create_with_seed(&payer_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        println!("Profile account pubkey: {}", profile_account_pubkey);
        let mut ixs = Vec::new();

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

        let pfp_pubkey = if let Some(ref pfp) = &params.pfp {
            Self::parse_pubkey(pfp)?
        } else {
            system_program::id()
        };

        let init_profile_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::CreatePlayerProfile {
                params: params.into(),
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(profile_account_pubkey, false),
                AccountMeta::new_readonly(pfp_pubkey, false),
            ],
        );

        ixs.push(init_profile_ix);

        let fee = self.get_recent_prioritization_fees(&[profile_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);
        ixs.insert(0, set_cu_prize_ix);

        let message = Message::new(&ixs, Some(&payer_pubkey));

        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    // TODO: add close_player_profile

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        let metaplex_program_id = Pubkey::from_str(METAPLEX_PROGRAM_ID).unwrap();

        if params.name.len() > MAX_NAME_LENGTH {
            return Err(TransportError::InvalidMetadataNameLength)?;
        }

        if params.symbol.len() > MAX_SYMBOL_LENGTH {
            return Err(TransportError::InvalidMetadataSymbolLength)?;
        }

        let (payer, payer_pubkey) = self.payer()?;

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

        let init_mint_ix = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_pubkey,
            &payer_pubkey,
            Some(&payer_pubkey),
            0,
        )
        .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;

        let (metadata_pda, _bump_seed) = Pubkey::find_program_address(
            &[
                "metadata".as_bytes(),
                metaplex_program_id.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &metaplex_program_id,
        );

        let (edition_pda, _bump_seed) = Pubkey::find_program_address(
            &[
                "metadata".as_bytes(),
                metaplex_program_id.as_ref(),
                mint_pubkey.as_ref(),
                "edition".as_bytes(),
            ],
            &metaplex_program_id,
        );

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
            AccountMeta::new_readonly(metaplex_program_id, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let publish_game_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::PublishGame {
                params: params.into(),
            },
            accounts,
        );

        let fee = self.get_recent_prioritization_fees(&[mint_pubkey, metadata_pda, edition_pda])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(
            &[
                set_cu_prize_ix,
                create_mint_account_ix,
                init_mint_ix,
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

    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        let SettleParams {
            addr,
            settles,
            transfer,
            awards,
            checkpoint,
            access_version,
            settle_version,
            next_settle_version,
            entry_lock,
            accept_deposits,
        } = params;
        info!("Settle game {}", addr);

        let (payer, payer_pubkey) = self.payer()?;
        let game_account_pubkey = Self::parse_pubkey(&addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;

        if game_state.settle_version != params.settle_version {
            return Err(Error::SettleVersionMismatch(
                params.settle_version,
                game_state.settle_version,
            ));
        }

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&game_account_pubkey.to_bytes()], &self.program_id);

        let recipient_account_pubkey = game_state.recipient_addr;
        let recipient_state = self
            .internal_get_recipient_state(&recipient_account_pubkey)
            .await?;

        let mut accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(Pubkey::from_str(&addr).unwrap(), false),
            AccountMeta::new(game_state.stake_account, false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(recipient_account_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let mut ix_settles: Vec<IxSettle> = Vec::new();
        let mut calc_cu_prize_addrs =
            vec![Pubkey::from_str(&addr).unwrap(), game_state.stake_account];

        for settle in settles.iter() {
            ix_settles.push(IxSettle {
                access_version: settle.player_id,
                withdraw: settle.withdraw,
                change: settle.change,
                eject: settle.eject,
            });

            let Some(player) = game_state
                .players
                .iter()
                .find(|p| p.access_version == settle.player_id)
            else {
                if settle.player_id == 0 && settle.withdraw == 0 {
                    continue;
                } else {
                    return Err(Error::InvalidSettle(format!(
                        "Player id {} in settle is invalid",
                        settle.player_id
                    )));
                }
            };

            // Append ATA when there's a payment
            if settle.withdraw > 0 {
                if is_native_mint(&game_state.token_mint) {
                    info!("Settle: add payment receiver wallet: {}", player.addr);
                    accounts.push(AccountMeta::new(player.addr, false));
                    calc_cu_prize_addrs.push(player.addr);
                } else {
                    let ata = get_associated_token_address(&player.addr, &game_state.token_mint);
                    info!("Settle: add payment receiver ATA: {}", ata);
                    accounts.push(AccountMeta::new(ata, false));
                    calc_cu_prize_addrs.push(ata);
                }
            }
        }

        if transfer.is_some() {
            if let Some(slot) = recipient_state
                .slots
                .iter()
                .find(|s| s.token_addr.eq(&game_state.token_mint))
            {
                info!("Settle: add slot stake account for transfer: {}", slot.stake_addr);
                accounts.push(AccountMeta::new(slot.stake_addr, false));
                calc_cu_prize_addrs.push(slot.stake_addr);
            }
        }

        for Award {
            player_id,
            bonus_identifier,
        } in awards.iter()
        {
            let Some(player) = game_state
                .players
                .iter()
                .find(|p| p.access_version == *player_id)
            else {
                return Err(Error::InvalidSettle(format!(
                    "Player id {} in award is invalid",
                    player_id
                )));
            };

            for bonus in game_state.bonuses.iter() {
                if bonus.identifier.eq(bonus_identifier) {
                    let bonus_addr = bonus.stake_addr.clone();
                    let receiver_addr =
                        get_associated_token_address(&player.addr, &bonus.token_addr);
                    info!("Settle: add bonus account: {}", bonus_addr);
                    accounts.push(AccountMeta::new(bonus_addr, false));
                    calc_cu_prize_addrs.push(bonus_addr);
                    info!("Settle: add bonus receiver: {}", receiver_addr);
                    accounts.push(AccountMeta::new(receiver_addr, false));
                    calc_cu_prize_addrs.push(receiver_addr);
                }
            }
        }

        info!("Solana transport settle game: {}\n  - Settle Version: {} -> {}\n  - Settles: {:?}\n  - Transfer: {:?}\n  - Awards: {:?}\n  - Checkpoint: {:?}",
            addr,
            settle_version,
            next_settle_version,
            ix_settles,
            transfer,
            awards,
            checkpoint
        );

        let params = RaceInstruction::Settle {
            params: IxSettleParams {
                settles: ix_settles,
                transfer,
                awards,
                checkpoint: borsh::to_vec(&checkpoint)?,
                access_version,
                settle_version,
                next_settle_version,
                entry_lock,
                accept_deposits,
            },
        };

        let set_cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(1200000);
        let fee = self.get_recent_prioritization_fees(&calc_cu_prize_addrs)?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let settle_ix = Instruction::new_with_borsh(self.program_id, &params, accounts);

        let message = Message::new(
            &[set_cu_prize_ix, set_cu_limit_ix, settle_ix],
            Some(&payer.pubkey()),
        );
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        let sig = self.send_transaction(tx)?;

        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;

        Ok(SettleResult {
            signature: sig.to_string(),
            game_account: game_state.into_account(addr)?,
        })
    }

    async fn reject_deposits(&self, params: RejectDepositsParams) -> Result<RejectDepositsResult> {
        let (payer, payer_pubkey) = self.payer()?;

        let game_account_pubkey = Self::parse_pubkey(&params.addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;

        let calc_cu_prize_addrs = vec![];

        let (pda, _) =
            Pubkey::find_program_address(&[game_account_pubkey.as_ref()], &self.program_id);

        let mut accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(game_account_pubkey, false),
            AccountMeta::new(game_state.stake_account.clone(), false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        for reject_deposit in params.reject_deposits.iter() {
            if let Some(deposit) = game_state
                .deposits
                .iter()
                .find(|d| d.access_version == *reject_deposit)
            {
                if is_native_mint(&game_state.token_mint) {
                    accounts.push(AccountMeta::new(deposit.addr.clone(), false));
                } else {
                    let ata = get_associated_token_address(&deposit.addr, &game_state.token_mint);
                    accounts.push(AccountMeta::new(ata, false));
                }
            } else {
                return Err(TransportError::InvalidRejectDeposits(*reject_deposit))?;
            }
        }

        let set_cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(1200000);
        let fee = self.get_recent_prioritization_fees(&calc_cu_prize_addrs)?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let ix_params = RaceInstruction::RejectDeposits {
            params: IxRejectDepositsParams {
                reject_deposits: params.reject_deposits,
            },
        };

        let reject_deposit_ix = Instruction::new_with_borsh(self.program_id, &ix_params, accounts);

        let message = Message::new(
            &[set_cu_prize_ix, set_cu_limit_ix, reject_deposit_ix],
            Some(&payer.pubkey()),
        );

        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[payer], blockhash);
        let sig = self.send_transaction(tx)?;
        Ok(RejectDepositsResult {
            signature: sig.to_string(),
        })
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        let (payer, payer_pubkey) = self.payer()?;
        let registry_account = Keypair::new();
        let registry_account_pubkey = registry_account.pubkey();
        let lamports = self.get_min_lamports(REGISTRY_INITIAL_ACCOUNT_LEN)?;
        info!("Needs {} lamports to make account rent-exempt.", lamports);
        let create_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &registry_account_pubkey,
            lamports,
            REGISTRY_INITIAL_ACCOUNT_LEN as u64,
            &self.program_id,
        );
        let create_registry_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::CreateRegistry {
                params: params.into(),
            },
            vec![
                AccountMeta::new_readonly(payer_pubkey, true),
                AccountMeta::new(registry_account_pubkey, false),
            ],
        );

        let fee = self.get_recent_prioritization_fees(&[registry_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(
            &[set_cu_prize_ix, create_account_ix, create_registry_ix],
            Some(&payer.pubkey()),
        );
        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[payer, &registry_account], blockhash);
        self.send_transaction(tx)?;
        let addr = registry_account_pubkey.to_string();
        Ok(addr)
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        let (payer, payer_pubkey) = self.payer()?;
        let recipient_account = Keypair::new();
        let recipient_account_pubkey = recipient_account.pubkey();
        let cap_pubkey = if let Some(addr) = params.cap_addr.as_ref() {
            Self::parse_pubkey(addr)?
        } else {
            payer_pubkey
        };
        let mut used_id = Vec::new();
        let mut init_token_accounts_ixs = Vec::new();
        let mut account_metas = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new_readonly(cap_pubkey, false),
            AccountMeta::new(recipient_account_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let lamports = self.get_min_lamports(RECIPIENT_ACCOUNT_LEN)?;

        let create_recipient_account_ix = system_instruction::create_account(
            &payer_pubkey,
            &recipient_account_pubkey,
            lamports,
            RECIPIENT_ACCOUNT_LEN as u64,
            &self.program_id,
        );

        let mut extra_signers = vec![];
        let mut slots = vec![];

        for slot in params.slots {
            if used_id.contains(&slot.id) {
                return Err(Error::InvalidRecipientSlotParams);
            }
            used_id.push(slot.id);

            let token_mint_pubkey = Self::parse_pubkey(&slot.token_addr)?;

            let init_shares = slot
                .init_shares
                .into_iter()
                .map(TryInto::try_into)
                .collect::<TransportResult<Vec<IxRecipientSlotShareInit>>>()?;

            if is_native_mint(&token_mint_pubkey) {
                // For SOL games, use PDA as stake account.
                let (pda, _) = Pubkey::find_program_address(
                    &[recipient_account_pubkey.as_ref(), &[slot.id]],
                    &self.program_id,
                );

                slots.push(IxRecipientSlotInit {
                    id: slot.id,
                    slot_type: slot.slot_type,
                    token_addr: Self::parse_pubkey(&slot.token_addr)?,
                    stake_addr: pda,
                    init_shares,
                });
                account_metas.push(AccountMeta::new(pda, false));
            } else {
                let stake_account = Keypair::new();
                // For SPL games, create a dedicated stake account
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

                init_token_accounts_ixs.push(create_stake_account_ix);
                init_token_accounts_ixs.push(init_stake_account_ix);
                account_metas.push(AccountMeta::new(stake_account_pubkey, false));

                slots.push(IxRecipientSlotInit {
                    id: slot.id,
                    slot_type: slot.slot_type,
                    token_addr: Self::parse_pubkey(&slot.token_addr)?,
                    stake_addr: stake_account_pubkey,
                    init_shares,
                });
                extra_signers.push(stake_account);
            }
        }

        let create_recipient_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::CreateRecipient {
                params: IxCreateRecipientParams { slots },
            },
            account_metas,
        );

        let mut signers = vec![payer, &recipient_account];
        let mut extra_signer_refs: Vec<&Keypair> = extra_signers.iter().collect();
        signers.append(&mut extra_signer_refs);

        let mut ixs = vec![create_recipient_account_ix];
        ixs.append(&mut init_token_accounts_ixs);
        ixs.push(create_recipient_ix);

        let message = Message::new(&ixs, Some(&payer.pubkey()));
        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&signers, blockhash);
        self.send_transaction(tx)?;
        Ok(recipient_account_pubkey.to_string())
    }

    async fn add_recipient_slot(&self, params: AddRecipientSlotParams) -> Result<String> {
        let AddRecipientSlotParams { recipient_addr, slot } = params;
        let (payer, payer_pubkey) = self.payer()?;

        let recipient_account_pubkey = Self::parse_pubkey(&recipient_addr)?;
        let recipient_state = self.internal_get_recipient_state(&recipient_account_pubkey).await?;
        let token_mint_pubkey = Self::parse_pubkey(&slot.token_addr)?;

        // Make sure it's a new slot ID and new token address
        if recipient_state.slots.iter().any(|s| s.id == slot.id || s.token_addr.eq(&token_mint_pubkey)) {
            return Err(Error::InvalidRecipientSlotParams);
        }

        let mut ixs = Vec::new();
        let mut account_metas = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(recipient_account_pubkey, false),
        ];

        let init_shares = slot
                .init_shares
                .into_iter()
                .map(TryInto::try_into)
                .collect::<TransportResult<Vec<IxRecipientSlotShareInit>>>()?;

        let slot_init: IxRecipientSlotInit;
        let mut extra_signers = vec![];

        if is_native_mint(&token_mint_pubkey) {
            // For SOL games, use PDA as stake account.
            let (pda, _) = Pubkey::find_program_address(
                &[recipient_account_pubkey.as_ref(), &[slot.id]],
                &self.program_id,
            );

            slot_init = IxRecipientSlotInit {
                id: slot.id,
                slot_type: slot.slot_type,
                token_addr: token_mint_pubkey,
                stake_addr: pda,
                init_shares,
            };
            account_metas.push(AccountMeta::new(pda, false));
        } else {
            let stake_account = Keypair::new();
            // For SPL games, create a dedicated stake account
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

            ixs.push(create_stake_account_ix);
            ixs.push(init_stake_account_ix);

            account_metas.push(AccountMeta::new(stake_account_pubkey, false));

            slot_init = IxRecipientSlotInit {
                id: slot.id,
                slot_type: slot.slot_type,
                token_addr: Self::parse_pubkey(&slot.token_addr)?,
                stake_addr: stake_account_pubkey,
                init_shares,
            };

            extra_signers.push(stake_account);
        }


        account_metas.push(AccountMeta::new_readonly(spl_token::id(), false));
        account_metas.push(AccountMeta::new_readonly(system_program::id(), false));

        let add_recipient_slot_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::AddRecipientSlot {
                params: slot_init,
            },
            account_metas,
        );

        let mut signers = vec![payer];
        let mut extra_signer_refs: Vec<&Keypair> = extra_signers.iter().collect();
        signers.append(&mut extra_signer_refs);

        ixs.push(add_recipient_slot_ix);

        let message = Message::new(&ixs, Some(&payer.pubkey()));
        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&signers, blockhash);
        self.send_transaction(tx)?;

        Ok(recipient_addr)
    }

    async fn assign_recipient(&self, _params: AssignRecipientParams) -> Result<()> {
        Ok(())
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        let reg_state = self
            .internal_get_registry_state(&reg_account_pubkey)
            .await?;
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
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let register_game_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::RegisterGame, // TODO: add is_hidden
            accounts,
        );

        let fee = self.get_recent_prioritization_fees(&[reg_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(&[set_cu_prize_ix, register_game_ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self.get_blockhash()?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;
        Ok(())
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;
        let game_account_pubkey = Self::parse_pubkey(&params.game_addr)?;
        let reg_account_pubkey = Self::parse_pubkey(&params.reg_addr)?;
        // let reg_state = self.get_registry_state(&reg_account_pubkey).await?;

        let accounts = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(reg_account_pubkey, false),
            AccountMeta::new_readonly(game_account_pubkey, false),
        ];

        let unregister_game_ix = Instruction::new_with_borsh(
            self.program_id,
            // TODO: add is_hidden param?
            &RaceInstruction::UnregisterGame,
            accounts,
        );

        let fee = self.get_recent_prioritization_fees(&[reg_account_pubkey])?;
        let set_cu_prize_ix = ComputeBudgetInstruction::set_compute_unit_price(fee);

        let message = Message::new(
            &[set_cu_prize_ix, unregister_game_ix],
            Some(&payer.pubkey()),
        );
        let mut tx = Transaction::new_unsigned(message);
        let blockhash = self
            .client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)?;
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }

    async fn subscribe_game_account<'a>(
        &'a self,
        addr: &'a str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        let ws_rpc = self
            .rpc
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let game_account_pubkey = Self::parse_pubkey(addr)?;
        let addr = addr.to_owned();

        let client = match PubsubClient::new(&ws_rpc).await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create PubsubClient due to {:?}", e);
                return Err(Error::TransportError(e.to_string()));
            }
        };

        Ok(Box::pin(stream! {
            let (mut stream, unsub) = match client
            .account_subscribe(
                &game_account_pubkey,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: Some(CommitmentConfig::finalized()),
                    min_context_slot: None,
                }),
            )
            .await
            {
                Ok((stream, unsub)) => (stream, unsub),
                Err(e) => {
                    error!("Failed on calling account_subscribe");
                    yield Err(Error::TransportError(e.to_string()));
                    return;
                }
            };

            while let Some(rpc_response) = stream.next().await {
                let ui_account = rpc_response.value;
                let Some(data) = ui_account.data.decode() else {
                    error!("Found an empty account data");
                    unsub().await;
                    return;
                };
                let state = match GameState::deserialize(&mut data.as_slice()) {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Game state deserialization error: {}", e.to_string());
                        yield(Err(Error::TransportError(e.to_string())));
                        unsub().await;
                        return;
                    }
                };
                let acc = match state.into_account(addr.clone()) {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Game account parsing error: {}", e.to_string());
                        yield(Err(Error::TransportError(e.to_string())));
                        unsub().await;
                        return;
                    }
                };
                yield(Ok(acc));
            }
        }))
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        let game_account_pubkey = Self::parse_pubkey(addr)?;
        let game_state = self.internal_get_game_state(&game_account_pubkey).await?;
        Ok(Some(game_state.into_account(addr)?))
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        let mint_pubkey = Self::parse_pubkey(addr)?;
        let metaplex_program_id = Pubkey::from_str(METAPLEX_PROGRAM_ID).unwrap();

        let (metadata_account_pubkey, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                metaplex_program_id.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &metaplex_program_id,
        );

        let metadata_account_data = self
            .client
            .get_account_data(&metadata_account_pubkey)
            .map_err(|e| TransportError::NetworkError(e.to_string()))?;
        let metadata_account_state =
            match metadata::Metadata::deserialize(&mut metadata_account_data.as_slice()) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to deserialize metadata account: {:?}", e);
                    return Err(TransportError::MetadataDeserializeError)?;
                }
            };
        let metadata_data = metadata_account_state.data;
        let uri = metadata_data.uri.trim_end_matches('\0').to_string();

        info!("Fetch wasm game bundle from {}", uri);

        let data = nft::fetch_wasm_from_game_bundle(&uri)
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))?;

        Ok(Some(GameBundle {
            addr: addr.to_string(),
            uri,
            name: metadata_data.name.trim_end_matches('\0').to_string(),
            data,
        }))
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        let wallet_pubkey = Self::parse_pubkey(addr)?;
        let profile_state = self.internal_get_player_state(&wallet_pubkey).await?;
        let pfp = profile_state.pfp.map(|x| x.to_string());
        Ok(Some(PlayerProfile {
            addr: addr.to_owned(),
            nick: profile_state.nick,
            pfp,
        }))
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        let wallet_pubkey = Self::parse_pubkey(addr)?;
        let server_state = self.internal_get_server_state(&wallet_pubkey).await?;

        Ok(Some(ServerAccount {
            addr: server_state.owner.to_string(),
            endpoint: server_state.endpoint,
        }))
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        let pubkey = Self::parse_pubkey(addr)?;
        let recipient_state = self.internal_get_recipient_state(&pubkey).await?;
        let stake_addrs: Vec<Pubkey> = recipient_state.slots.iter().map(|s| s.stake_addr).collect();
        let mut recipient_account = recipient_state.into_account(addr);
        // Add amount information by querying stake accounts
        for (i, stake_addr) in stake_addrs.iter().enumerate() {
            tracing::info!("Check stake account: {}", stake_addr);
            let slot = recipient_account
                .slots
                .get_mut(i)
                .ok_or(Error::TransportError(
                    "[Unreachable] Cannot get recipient".into(),
                ))?;
            if is_native_mint(&parse_pubkey(&slot.token_addr)?) {
                println!("stake addr: {}", stake_addr);
                slot.balance = self.client.get_balance(stake_addr)
                    .or(Err(Error::TransportError("Cannot get the state of stake account(SOL)".into())))?;
            } else {
                let stake_account =
                    self.client
                        .get_account_data(stake_addr)
                        .or(Err(Error::TransportError(
                            "Cannot get the state of stake account(SPL)".into(),
                        )))?;
                let stake_account_state = Account::unpack(&stake_account).or(Err(
                    Error::TransportError("Cannot parse data of stake account".into()),
                ))?;
                slot.balance = stake_account_state.amount;
            }
        }
        Ok(Some(recipient_account))
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        let key = Self::parse_pubkey(addr)?;
        let state = self.internal_get_registry_state(&key).await?;

        Ok(Some(RegistrationAccount {
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
                    bundle_addr: g.bundle_addr.to_string(),
                })
                .collect(),
        }))
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        let (payer, payer_pubkey) = self.payer()?;
        let recipient_pubkey = Self::parse_pubkey(&params.recipient_addr)?;
        let recipient_state = self.internal_get_recipient_state(&recipient_pubkey).await?;


        let mut account_metas = vec![
            AccountMeta::new_readonly(payer_pubkey, true),
            AccountMeta::new(recipient_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        for slot in recipient_state.slots.iter() {
            for slot_share in slot.shares.iter() {
                match slot_share.owner {
                    RecipientSlotOwner::Assigned { ref addr } if addr.eq(&payer_pubkey) => {
                        let (pda, _) =
                            Pubkey::find_program_address(&[&recipient_pubkey.to_bytes(), &[slot.id]], &self.program_id);

                        account_metas.push(AccountMeta::new_readonly(pda, false));
                        account_metas.push(AccountMeta::new(slot.stake_addr, false));
                        info!(
                            "Expect to claim tokens from slot {}, token address: {}",
                            slot.id, slot.token_addr
                        );
                        if is_native_mint(&slot.token_addr) {
                            info!("Receiver: {}", payer.pubkey());
                            account_metas.push(AccountMeta::new(payer.pubkey(), false));
                        } else {
                            let ata = get_associated_token_address(addr, &slot.token_addr);
                            info!("Receiver: {}", ata);
                            account_metas.push(AccountMeta::new(ata, false));
                        }
                    }
                    _ => (),
                }
            }
        }

        if account_metas.len() == 5 {
            return Err(Error::TransportError("No slot to claim".into()));
        }

        let recipient_claim_ix = Instruction::new_with_borsh(
            self.program_id,
            &RaceInstruction::RecipientClaim,
            account_metas,
        );

        let message = Message::new(&[recipient_claim_ix], Some(&payer_pubkey));

        let blockhash = self.get_blockhash()?;
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[payer], blockhash);
        self.send_transaction(tx)?;

        Ok(())
    }
}

impl SolanaTransport {
    pub fn try_new(
        rpc: String,
        keyfile: Option<PathBuf>,
        skip_preflight: bool,
    ) -> TransportResult<Self> {
        let keypair = keyfile.map(read_keypair).transpose()?;
        let program_id = Pubkey::from_str(PROGRAM_ID)?;
        SolanaTransport::try_new_with_program_id(rpc, keypair, program_id, skip_preflight)
    }

    pub(crate) fn payer(&self) -> TransportResult<(&Keypair, Pubkey)> {
        if let Some(ref keypair) = self.keypair {
            Ok((keypair, keypair.pubkey()))
        } else {
            Err(TransportError::UnspecifiedSigner)
        }
    }

    pub(crate) fn try_new_with_program_id(
        rpc: String,
        keypair: Option<Keypair>,
        program_id: Pubkey,
        skip_preflight: bool,
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
        let debug = skip_preflight;
        let client = RpcClient::new_with_timeout_and_commitment(
            rpc.clone(),
            Duration::from_secs(60),
            commitment,
        );
        Ok(Self {
            rpc,
            client,
            keypair,
            program_id,
            debug,
        })
    }

    fn parse_pubkey(addr: &str) -> TransportResult<Pubkey> {
        Pubkey::from_str(addr)
            .map_err(|_| TransportError::InvalidConfig(format!("Can't parse public key: {}", addr)))
    }

    fn get_recent_prioritization_fees(&self, pubkeys: &[Pubkey]) -> TransportResult<u64> {
        let fees = self
            .client
            .get_recent_prioritization_fees(pubkeys)
            .map_err(|e| TransportError::FeeCalculationError(e.to_string()))?;
        let mut fee = 0;
        for f in fees {
            fee += f.prioritization_fee;
        }
        info!("Estimate fee: {} in lamports", fee);
        // XXX: We add a fixed amount to recommended fee
        Ok(fee + 10000)
    }

    fn get_min_lamports(&self, account_len: usize) -> TransportResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(account_len)
            .map_err(|_| TransportError::FailedToGetMinimumLamports)
    }

    fn get_blockhash(&self) -> TransportResult<Hash> {
        self.client
            .get_latest_blockhash()
            .map_err(|_| TransportError::GetBlockhashFailed)
    }

    fn send_transaction(&self, tx: Transaction) -> TransportResult<Signature> {
        let confirm_num = if cfg!(test) { 1 } else { 32 };

        let config = RpcSendTransactionConfig {
            skip_preflight: self.debug,
            // skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        };

        let simulation_result = self.client.simulate_transaction(&tx);
        match simulation_result {
            Ok(rst) => {
                if let Some(e) = rst.value.err {
                    if let Some(logs) = rst.value.logs {
                        for log in logs {
                            warn!("{}", log);
                        }
                    }
                    error!("Simulation failed due to error: {:?}", e);
                }
            }
            Err(e) => {
                error!("Simulation error: {:?}", e);
            }
        }

        let sig = self
            .client
            .send_transaction_with_config(&tx, config)
            .map_err(|e| {
                if let Some(e) = e.get_transaction_error() {
                    error!("Transactior error: {}", e);
                    TransportError::ClientSendTransactionFailed(e.to_string())
                } else {
                    TransportError::ClientSendTransactionFailed(e.to_string())
                }
            })?;

        let num_confirmed = self.client
            .poll_for_signature_confirmation(&sig, confirm_num)
            .map_err(|e| TransportError::ClientSendTransactionFailed(e.to_string()))?;

        info!("Transaction confirmed. {} confirmations passed.", num_confirmed);

        match self
            .client
            .get_transaction(&sig, UiTransactionEncoding::Json)
        {
            Ok(tx) => {
                if let Some(meta) = tx.transaction.meta {
                    meta.log_messages.map(|log_messages| {
                        for log_message in log_messages {
                            info!("{}", log_message);
                        }
                    });
                }
            }
            Err(e) => {
                if let Some(e) = e.get_transaction_error() {
                    error!("Error: {:?}", e);
                }
            }
        }
        Ok(sig)
    }

    /// Get the state of an on-chain game account by its public key
    /// Not for public API usage
    async fn internal_get_game_state(
        &self,
        game_account_pubkey: &Pubkey,
    ) -> TransportResult<GameState> {
        let game_account = self
            .client
            .get_account_with_commitment(game_account_pubkey, CommitmentConfig::finalized())
            .map_err(|e| TransportError::AccountNotFound(e.to_string()))?
            .value
            .ok_or(TransportError::AccountNotFound("".to_string()))?;
        // TODO: complete error message

        GameState::deserialize(&mut game_account.data.as_slice())
            .map_err(|_| TransportError::GameStateDeserializeError)
    }

    /// Get the state of an on-chain recipient account by its public key.
    /// Not for public API usage
    async fn internal_get_recipient_state(
        &self,
        recipient_account_pubkey: &Pubkey,
    ) -> TransportResult<RecipientState> {
        let data = self
            .client
            .get_account_data(recipient_account_pubkey)
            .or(Err(TransportError::RecipientAccountNotFound))?;
        RecipientState::deserialize(&mut data.as_slice())
            .map_err(|_| TransportError::RecipientStateDeserializeError)
    }

    /// Get the state of an on-chain server account by its public key.
    /// Not for public API usage
    #[allow(dead_code)]
    async fn internal_get_server_state(
        &self,
        server_pubkey: &Pubkey,
    ) -> TransportResult<ServerState> {
        let server_account_pubkey =
            Pubkey::create_with_seed(server_pubkey, SERVER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let data = self
            .client
            .get_account_data(&server_account_pubkey)
            .or(Err(TransportError::ServerAccountDataNotFound))?;
        ServerState::deserialize(&mut data.as_slice())
            .map_err(|_| TransportError::ServerStateDeserializeError)
    }

    async fn internal_get_player_state(
        &self,
        player_pubkey: &Pubkey,
    ) -> TransportResult<PlayerState> {
        let profile_pubkey =
            Pubkey::create_with_seed(player_pubkey, PLAYER_PROFILE_SEED, &self.program_id)
                .map_err(|_| TransportError::AddressCreationFailed)?;

        let data = self
            .client
            .get_account_data(&profile_pubkey)
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
            .get_account_data(registry_account_pubkey)
            .or(Err(TransportError::RegistryAccountDataNotFound))?;

        RegistryState::deserialize(&mut data.as_slice())
            .map_err(|_| TransportError::RegistryStateDeserializeError)
    }
}

impl From<ParsePubkeyError> for TransportError {
    fn from(_: ParsePubkeyError) -> Self {
        TransportError::ParseAddressError
    }
}

#[cfg(test)]
mod tests {
    use race_core::types::EntryType;

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
    #[ignore]
    fn test_read_program_id() -> anyhow::Result<()> {
        read_program_id()?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_read_keypair() -> anyhow::Result<()> {
        let _keypair = read_keypair(
            shellexpand::tilde("~/.config/solana/id.json")
                .to_string()
                .into(),
        )?;
        Ok(())
    }

    fn get_transport() -> anyhow::Result<SolanaTransport> {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        let transport = SolanaTransport::try_new_with_program_id(
            "http://localhost:8899".into(),
            keypair,
            read_program_id()?,
            true,
        )?;
        transport.client.request_airdrop(&pubkey, 1_000_000_000)?;
        Ok(transport)
    }

    #[test]
    #[ignore]
    fn test_get_transport() -> anyhow::Result<()> {
        get_transport()?;
        Ok(())
    }

    async fn create_player(transport: &SolanaTransport) -> anyhow::Result<()> {
        let _player = transport
            .create_player_profile(CreatePlayerProfileParams {
                nick: "Alice".to_string(),
                pfp: None,
            })
            .await?;
        Ok(())
    }

    async fn create_game(transport: &SolanaTransport) -> anyhow::Result<String> {
        let addr = transport
            .create_game_account(CreateGameAccountParams {
                title: "16-CHAR_GAME_TIL".to_string(),
                bundle_addr: "6CGkN7T2JXdh9zpFumScSyRtBcyMzBM4YmhmnrYPQS5w".to_owned(),
                token_addr: NATIVE_MINT.to_string(),
                max_players: 9,
                data: Vec::<u8>::new(),
                entry_type: EntryType::Cash {
                    min_deposit: 10,
                    max_deposit: 20,
                },
                recipient_addr: "1111111111111111111111111111".to_string(),
            })
            .await?;
        println!("Create game at {}", addr);
        Ok(addr)
    }

    async fn create_reg(_transport: &SolanaTransport) -> anyhow::Result<String> {
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
    #[ignore]
    async fn test_game_create_get_close() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = create_game(&transport).await?;
        let game_account = transport
            .get_game_account(&addr)
            .await?
            .expect("Failed to query");
        assert_eq!(game_account.access_version, 0);
        assert_eq!(game_account.settle_version, 0);
        assert_eq!(game_account.max_players, 9);
        assert_eq!(game_account.title, "16-CHAR_GAME_TIL");
        transport
            .close_game_account(CloseGameAccountParams { addr: addr.clone() })
            .await
            .expect("Failed to close");
        assert_eq!(None, transport.get_game_account(&addr).await?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_registry_create_get() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let addr = create_reg(&transport).await?;
        let reg = transport.get_registration(&addr).await?.unwrap();
        assert_eq!(reg.is_private, false);
        assert_eq!(reg.size, 100);
        assert_eq!(reg.games.len(), 0);
        let game_addr = create_game(&transport).await?;
        transport
            .register_game(RegisterGameParams {
                game_addr: game_addr.clone(),
                reg_addr: addr.clone(),
            })
            .await?;
        let reg = transport.get_registration(&addr).await?.unwrap();
        assert_eq!(reg.games.len(), 1);
        transport
            .unregister_game(UnregisterGameParams {
                game_addr,
                reg_addr: addr.clone(),
            })
            .await?;
        let reg = transport.get_registration(&addr).await?.unwrap();
        assert_eq!(reg.games.len(), 0);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_register_server() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let endpoint = "https://foo.bar".to_string();
        let _addr = transport
            .register_server(RegisterServerParams {
                endpoint: endpoint.clone(),
            })
            .await?;

        let server = transport
            .get_server_account(&transport.wallet_pubkey().to_string())
            .await?
            .unwrap();
        assert_eq!(server.addr, transport.wallet_pubkey().to_string());
        assert_eq!(server.endpoint, endpoint);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
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
            .await?
            .unwrap();
        assert_eq!(profile.addr, transport.wallet_pubkey().to_string());
        assert_eq!(profile.nick, nick);
        assert_eq!(profile.pfp, None);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_serve_game() -> anyhow::Result<()> {
        let transport = get_transport()?;
        let game_addr = create_game(&transport).await?;
        let _server_addr = transport
            .serve(ServeParams {
                game_addr: game_addr.clone(),
                verify_key: "VERIFY KEY".into(),
            })
            .await?;
        let game = transport
            .get_game_account(&game_addr)
            .await?
            .expect("Failed to get game");
        assert_eq!(game.servers.len(), 1);
        assert_eq!(
            game.transactor_addr,
            Some(transport.wallet_pubkey().to_string())
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_join() -> anyhow::Result<()> {
        let transport = get_transport()?;
        create_player(&transport).await?;
        let game_addr = create_game(&transport).await?;
        println!("Join game: {}", game_addr);
        transport
            .join(JoinParams {
                game_addr: game_addr.clone(),
                amount: 500_000_000u64,
                access_version: 0u64,
                position: 0u16,
                verify_key: "VERIFY KEY".into(),
            })
            .await?;

        let game = transport
            .get_game_account(&game_addr)
            .await?
            .expect("Failed to get game");
        assert_eq!(game.players.len(), 1);

        let transport = get_transport()?;
        create_player(&transport).await?;
        println!("Join game: {}", game_addr);
        transport
            .join(JoinParams {
                game_addr: game_addr.clone(),
                amount: 500_000_000u64,
                access_version: 0u64,
                position: 0u16,
                verify_key: "VERIFY KEY".into(),
            })
            .await?;
        let game = transport
            .get_game_account(&game_addr)
            .await?
            .expect("Failed to get game");
        assert_eq!(game.players.len(), 2);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
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
            .await?
            .expect("Failed to get game bundle");

        assert_eq!(bundle.name, "RACE_raffle".to_string());
        assert_eq!(
            bundle.uri,
            "https://arweave.net/uQFXQ9Jp5IrO5qGuTX8zSWRMJU679M6ZGW9MM1cSP0E".to_string()
        );
        Ok(())
    }

    #[allow(dead_code)]
    #[ignore]
    async fn test_settle() -> anyhow::Result<()> {
        // let game_addr = create_game();
        Ok(())
    }
}
