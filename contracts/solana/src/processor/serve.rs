///! Server joins a game
///!
///! When a server joins an on-chain game, it can be either of the following cases:
///!
///! 1. It is the first to join and thus it also becomes the transactor
///! 2. It is the nth to join and n is less than or equal to 10
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
    state::{GameState, Padded, ServerJoin, ServerState},
};
use race_solana_types::constants::MAX_SERVER_NUM;

#[inline(never)]
pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    let payer_account = next_account_info(account_iter)?;
    if !payer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let game_account = next_account_info(account_iter)?;
    if !game_account.is_writable {
        return Err(ProcessError::InvalidAccountStatus)?;
    }

    let mut game_state = GameState::unpack(&game_account.try_borrow_mut_data()?)?;
    if !game_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    let server_account = next_account_info(account_iter)?;
    let server_state = ServerState::unpack(&server_account.try_borrow_data()?)?;
    if !server_state.is_initialized {
        return Err(ProcessError::DuplicateServerJoin)?;
    }
    if game_state.servers.len() == MAX_SERVER_NUM {
        return Err(ProcessError::ServerNumberExceedsLimit)?;
    }

    if game_state
        .servers
        .iter()
        .any(|s| s.addr.eq(server_account.key))
    {
        return Err(ProcessError::DuplicateServerJoin)?;
    }

    let new_access_version = game_state.access_version + 1;
    let server_to_join = ServerJoin {
        addr: *payer_account.key,
        endpoint: server_state.endpoint.clone(),
        access_version: new_access_version,
    };

    if game_state.transactor_addr.is_none() || game_state.servers.len() == 0 {
        msg!("Serve as transactor: {}", server_account.key);
        game_state.transactor_addr = Some(*payer_account.key);
    }

    game_state.servers.push(server_to_join);
    game_state.access_version = new_access_version;
    game_state.update_padding()?;

    msg!(
        "Server {} joins game {}",
        payer_account.key,
        game_account.key
    );

    GameState::pack(game_state, &mut game_account.try_borrow_mut_data()?)?;
    Ok(())
}
