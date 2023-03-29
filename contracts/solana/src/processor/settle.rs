//! Settle game result
//!
//! Transfer the game assets between players, eject and payout leaving players.
//! This instruction is only available for current game transactor.
//!
//! Settles must be validated:
//! 1. All changes are sum up to zero.
//! 2. Player without assets must be ejected.

use crate::{error::ProcessError, state::GameState};
use race_solana_types::types::{SettleOp, SettleParams};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use super::misc::{validate_receiver_account, TransferSource};

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: SettleParams,
) -> ProgramResult {
    let SettleParams { mut settles } = params;

    let account_iter = &mut accounts.iter();

    let transactor_account = next_account_info(account_iter)?;

    let game_account = next_account_info(account_iter)?;

    let stake_account = next_account_info(account_iter)?;

    let pda_account = next_account_info(account_iter)?;

    let token_program = next_account_info(account_iter)?;

    let system_program = next_account_info(account_iter)?;

    if !transactor_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Ensure changes are sum up to zero
    let mut sum = 0i64;

    // Collect the payouts.
    let mut payouts: Vec<(Pubkey, u64)> = Vec::new();

    // We should check the order of settles: add < sub < ejec
    // 0 for add, 1 for sub, 2 for eject.
    let mut op_type = 0;
    let mut game_state = GameState::unpack(&game_account.try_borrow_mut_data()?)?;

    if stake_account.key.ne(&game_state.stake_addr) {
        return Err(ProcessError::InvalidStakeAccount)?;
    }

    for settle in settles.into_iter() {
        match settle.op {
            SettleOp::Add(amt) => {
                if op_type != 0 {
                    return Err(ProcessError::InvalidOrderOfSettles)?;
                }
                if let Some(player) = game_state
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&settle.addr))
                {
                    player.balance = player
                        .balance
                        .checked_add(amt)
                        .ok_or(ProcessError::PlayerBalanceOverflow)?;
                } else {
                    return Err(ProcessError::InvalidSettlePlayerAddress)?;
                }
                sum += amt as i64;
            }
            SettleOp::Sub(amt) => {
                if op_type == 2 {
                    return Err(ProcessError::InvalidOrderOfSettles)?;
                }
                if let Some(player) = game_state
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&settle.addr))
                {
                    player.balance = player
                        .balance
                        .checked_sub(amt)
                        .ok_or(ProcessError::PlayerBalanceOverflow)?;
                } else {
                    return Err(ProcessError::InvalidSettlePlayerAddress)?;
                }
                sum -= amt as i64;
                op_type = 1;
            }
            SettleOp::Eject => {
                if op_type == 0 {
                    return Err(ProcessError::InvalidOrderOfSettles)?;
                }
                let idx = game_state
                    .players
                    .iter()
                    .position(|p| p.addr.eq(&settle.addr));
                if let Some(idx) = idx {
                    let player = game_state.players.remove(idx);
                    payouts.push((player.addr, player.balance));
                } else {
                    return Err(ProcessError::InvalidSettlePlayerAddress)?;
                }
                op_type = 2;
            }
        }
    }

    if sum != 0 {
        return Err(ProcessError::InvalidSettleAmounts)?;
    }

    // Ensure all players' assets are greater than zero
    for player in game_state.players.iter() {
        if player.balance == 0 {
            return Err(ProcessError::UnhandledEliminatedPlayer)?;
        }
    }

    // Payout tokens
    let transfer_source = TransferSource::try_new(
        system_program.clone(),
        token_program.clone(),
        stake_account.clone(),
        game_account.key.as_ref(),
        pda_account.clone(),
        program_id,
    )?;

    for (addr, amount) in payouts.into_iter() {
        let receiver_ata = next_account_info(account_iter)?;
        validate_receiver_account(&addr, &game_state.token_addr, receiver_ata.key)?;
        transfer_source.transfer(receiver_ata, amount)?;
    }

    GameState::pack(game_state, &mut game_account.try_borrow_mut_data()?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::*;
}
