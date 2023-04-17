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
    state::{Padded, ServerState},
};
use race_solana_types::constants::SERVER_PROFILE_SEED;
use race_solana_types::types::RegisterServerParams;

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: RegisterServerParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let owner_account = next_account_info(account_iter)?;
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let server_account = next_account_info(account_iter)?;
    if !server_account.is_writable {
        return Err(ProcessError::InvalidAccountStatus)?;
    }

    let server_pubkey =
        Pubkey::create_with_seed(owner_account.key, SERVER_PROFILE_SEED, program_id)?;
    if server_pubkey != *server_account.key {
        return Err(ProcessError::InvalidAccountPubkey)?;
    }

    let mut server_state = ServerState {
        is_initialized: true,
        addr: server_account.key.clone(),
        owner: *owner_account.key,
        endpoint: params.endpoint,
        padding: Default::default(),
    };

    server_state.update_padding()?;

    msg!("Server state: {:?}", &server_state);

    ServerState::pack(server_state, &mut server_account.try_borrow_mut_data()?)?;

    msg!("Server addr: {:?}", server_account.key);

    Ok(())
}
