use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use spl_token::{
    instruction::{set_authority, AuthorityType},
    state::Mint,
};

use crate::{state::{GameState, PlayerJoin}, instruction::CreateGameAccountParams};

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: CreateGameAccountParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    // let transactor_account = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;
    let temp_stake_account = next_account_info(account_iter)?;
    let token_account = next_account_info(account_iter)?;
    // let owner_account = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, _bump_seed) = Pubkey::find_program_address(&[game_account.key.as_ref()], program_id);
    let set_authority_ix = set_authority(
        token_program.key,
        temp_stake_account.key,
        Some(&pda),
        AuthorityType::AccountOwner,
        payer.key,
        &[&payer.key],
    )?;

    invoke(
        &set_authority_ix,
        &[
            temp_stake_account.clone(),
            payer.clone(),
            token_program.clone(),
        ],
    )?;
    msg!("1");
    msg!("Game acct len {}", game_account.data_len());
    msg!("Game acct data {:?}", game_account.try_borrow_data()?);
    let mut game_state = GameState::unpack_unchecked(&game_account.try_borrow_data()?)?;
    msg!("2");
    if game_state.is_initialized {
        msg!("The game already exists on chain!");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let token_state = Mint::unpack_unchecked(&token_account.data.borrow())?;
    if !token_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    msg!("3");

    game_state.is_initialized = true;
    game_state.title = params.title;
    game_state.owner = payer.key.clone();
    game_state.transactor_addr = None;
    game_state.settle_version = 0;
    game_state.access_version = 0;
    game_state.players = Box::new(Vec::<PlayerJoin>::with_capacity(params.max_players as usize));
    game_state.servers = Box::new(vec![]);
    game_state.max_players = params.max_players;
    game_state.data_len = params.data.len() as u32;
    game_state.data = Box::new(params.data);

    game_state.update_padding();
    msg!("4");

    // let test_vec = game_state.try_to_vec().unwrap();
    // msg!("Game state len {}", test_vec.len());
    GameState::pack(game_state, &mut game_account.try_borrow_mut_data()?)?;
    msg!("5");

    Ok(())
}
