// use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use crate::{
    error::RaceError,
    state::{GameState, PlayerJoin},
};
use race_core::types::CreateGameAccountParams;
use spl_token::{
    instruction::{set_authority, AuthorityType},
    state::Mint,
};

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: CreateGameAccountParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;
    let temp_stake_account = next_account_info(account_iter)?;
    let token_account = next_account_info(account_iter)?;
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
    let token_state = Mint::unpack_unchecked(&token_account.data.borrow())?;
    if !token_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    // let bundle_addr = Pubkey::try_from(params.bundle_addr.as_bytes())
    //     .map_err(|_| RaceError::UnpackOptionFailed)
    //     .unwrap();

    let mut game_state = GameState {
        is_initialized: true,
        title: params.title,
        // bundle_addr,
        owner: payer.key.clone(),
        transactor_addr: None,
        access_version: 0,
        settle_version: 0,
        max_players: params.max_players,
        data_len: params.data.len() as u32,
        data: Box::new(params.data),
        players: Box::new(Vec::<PlayerJoin>::with_capacity(
            params.max_players as usize,
        )),
        servers: Box::new(vec![]),
        padding: Box::new(vec![]),
    };

    msg!("2");

    game_state.update_padding();

    GameState::pack(game_state, &mut game_account.try_borrow_mut_data()?)?;
    msg!("On chain game account sucessfully created and initialized!");

    Ok(())
}
