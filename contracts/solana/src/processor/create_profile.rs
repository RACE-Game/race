use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use crate::{error::ProcessError, state::{PlayerState, Padded}};
use race_solana_types::constants::PROFILE_SEED;
use race_solana_types::types::CreatePlayerProfileParams;

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: CreatePlayerProfileParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let owner_account = next_account_info(account_iter)?;

    let profile_account = next_account_info(account_iter)?;

    let profile_pubkey = Pubkey::create_with_seed(owner_account.key, PROFILE_SEED, program_id)?;

    let pfp_account = next_account_info(account_iter)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !profile_account.is_writable {
        return Err(ProcessError::InvalidAccountStatus)?;
    }

    if profile_pubkey != *profile_account.key {
        return Err(ProcessError::InvalidAccountPubkey)?;
    }

    let rent = Rent::default();
    if !rent.is_exempt(profile_account.lamports(), PlayerState::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    let pfp_pubkey = Some(pfp_account.key.clone());

    let mut player_state = PlayerState {
        is_initialized: true,
        owner: *owner_account.key,
        addr: *profile_account.key,
        chips: 0u64,
        nick: params.nick,
        pfp: pfp_pubkey,
        padding: Default::default(),
    };

    player_state.update_padding()?;

    msg!("player profile state: {:?}", &player_state);

    PlayerState::pack(player_state, &mut profile_account.try_borrow_mut_data()?)?;

    msg!("Profile addr: {:?}", profile_account.key);

    Ok(())
}
