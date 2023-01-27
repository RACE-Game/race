use borsh::BorshSerialize;
use race_core::types::{CreateGameAccountParams, GameAccount, PlayerDeposit, PlayerJoin};
use solana_program::program::invoke_signed;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
};
use spl_token::{
    instruction::{close_account, transfer},
    state::Account,
};
use std::mem;

use crate::instruction::game_account_seed;

pub fn process_join_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: crate::instruction::JoinGameParams,
) -> ProgramResult {
    let account_iter = &mut accounts.into_iter();

    let player_account = next_account_info(account_iter)?;

    let temp_account = next_account_info(account_iter)?;

    let pda_account = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;

    // if mint is native
    // TODO: needs something like this
    // https://github.com/RACE-Game/racepoker-dealer/blob/cfd4a25ab606b385d9b950fe7acc4cc4ef76a8b1/src/processor/buyin.rs
    if true {
        let close_temp_account_ix = close_account(
            token_program.key,
            temp_account.key,
            pda_account.key,
            player_account.key,
            &[&player_account.key],
        )?;
        invoke(
            &close_temp_account_ix,
            &[
                temp_account.clone(),
                pda_account.clone(),
                player_account.clone(),
            ],
        )?;
    }
    msg!("OK");
    Ok(())
}
