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
    state::{GameState, ServerJoin, ServerState},
};

#[inline(never)]
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let owner_account = next_account_info(account_iter)?;
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("1");
    let game_account = next_account_info(account_iter)?;
    if !game_account.is_writable {
        return Err(ProcessError::InvalidAccountStatus)?;
    }
    msg!("2");
    let server_account = next_account_info(account_iter)?;

    let mut game_state = GameState::unpack(&game_account.try_borrow_mut_data()?)?;
    if game_state
        .servers
        .iter()
        .any(|s| s.addr.eq(server_account.key))
    {
        return Err(ProcessError::DuplicateServerJoin)?;
    }
    msg!("3");

    let server_state = ServerState::unpack(&server_account.try_borrow_data()?)?;

    let new_access_version = game_state.access_version + 1;
    let server_to_join = ServerJoin {
        addr: *server_account.key,
        endpoint: server_state.endpoint.clone(),
        access_version: new_access_version,
    };
    msg!("4");

    if game_state.transactor_addr.is_none() || game_state.servers.len() == 0 {
        game_state.transactor_addr = Some(*server_account.key);
    }
    game_state.servers.push(server_to_join);
    game_state.access_version = new_access_version;
    game_state.update_padding();
    msg!("5");

    msg!("Game state: {:?}", &game_state);

    Ok(())
}
