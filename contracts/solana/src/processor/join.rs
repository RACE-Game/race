///! Player joins a game (cash, sng or tourney)

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use crate::{
    error::ProcessError,
    state::{GameState, PlayerJoin, Padded},
};
use race_solana_types::types::JoinParams;

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: JoinParams
) -> ProgramResult {

    Ok(())
}
