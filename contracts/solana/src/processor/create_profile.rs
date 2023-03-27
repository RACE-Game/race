use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use race_solana_types::types::CreatePlayerProfileParams;
use race_solana_types::constants::PROFILE_SEED;
use crate::{state::PlayerState, error::ProcessError};

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: CreatePlayerProfileParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let player_account = next_account_info(account_iter)?;
    if !player_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let profile_account = next_account_info(account_iter)?;
    if !profile_account.is_writable {
        return Err(ProcessError::InvalidAccountStatus)?;
    }

    let profile_pubkey =
        Pubkey::create_with_seed(player_account.key, PROFILE_SEED, program_id)?;
    if profile_pubkey != *profile_account.key {
        return Err(ProcessError::InvalidAccountPubkey)?;
    }

    let pfp_account = next_account_info(account_iter)?;
    let pfp_pubkey = Some(pfp_account.key.clone());

    // TODO: Check rent exemption?

    let mut player_state = PlayerState {
        is_initialized: true,
        addr: *player_account.key,
        chips: 0u64,
        nick: params.nick,
        pfp: pfp_pubkey,
        padding: Vec::<u8>::new(),
    };

    player_state.update_padding();

    msg!("player profile state: {:?}", &player_state);

    PlayerState::pack(player_state, &mut profile_account.try_borrow_mut_data()?)?;

    msg!("Newly created prrofile addr: {:?}", profile_account.key);

    Ok(())
}
