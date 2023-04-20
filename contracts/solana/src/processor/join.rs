use crate::{
    error::ProcessError,
    state::{GameState, PlayerJoin},
};
use race_solana_types::types::JoinParams;
///! Player joins a game (cash, sng or tourney)
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    // system_instruction::transfer as system_transfer,
};
use spl_token::{
    instruction::{close_account, transfer},
    native_mint,
    state::Account,
};

#[inline(never)]
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], params: JoinParams) -> ProgramResult {
    let account_iter = &mut accounts.into_iter();

    let payer_account = next_account_info(account_iter)?;

    let player_account = next_account_info(account_iter)?;

    let temp_account = next_account_info(account_iter)?;

    let game_account = next_account_info(account_iter)?;

    let mint_account = next_account_info(account_iter)?;

    let stake_account = next_account_info(account_iter)?;

    let pda_account = next_account_info(account_iter)?;

    let token_program = next_account_info(account_iter)?;

    if !payer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let rent = Rent::default();

    if !Rent::is_exempt(&rent, player_account.lamports(), player_account.data_len()) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    let mut game_state = GameState::unpack(&game_account.try_borrow_data()?)?;

    if game_state.stake_account.ne(stake_account.key) {
        return Err(ProgramError::InvalidArgument);
    }

    if game_state.token_mint.ne(mint_account.key) {
        return Err(ProcessError::InvalidMint)?;
    }

    // 1. game already full?
    // 2. position within [0..=(len-1)]?
    // 3. player already joined?
    // 4. position already taken?
    if game_state.max_players as usize == game_state.players.len() {
        return Err(ProcessError::GameFullAlready)?;
    }

    if game_state.players.len() == 0 && params.position != 0 {
        return Err(ProcessError::InvalidPosition)?;
    }

    if game_state
        .players
        .iter()
        .any(|p| p.addr == *player_account.key)
    {
        return Err(ProcessError::JoinedGameAlready)?;
    }

    if game_state
        .players
        .iter()
        .any(|p| p.position == params.position as u32)
    {
        return Err(ProcessError::PositionTakenAlready)?;
    }

    msg!("Player position: {:?}", params.position);

    // TODO: Check game status?
    // if game_state.status != GameStatus::Open {
    //     return Err(DealerError::InvalidGameStatus)?;
    // }

    // Increase game access version
    game_state.access_version += 1;

    // Player joins
    game_state.players.push(PlayerJoin {
        addr: player_account.key.clone(),
        balance: params.amount,
        position: params.position as _,
        access_version: game_state.access_version,
    });

    // Check player's deposit
    if params.amount < game_state.min_deposit || params.amount > game_state.max_deposit {
        return Err(ProcessError::InvalidDeposit)?;
    }

    // Transfer player deposit to game stake account
    let temp_state = Account::unpack(&temp_account.try_borrow_data()?)?;

    if game_state.token_mint.ne(&native_mint::id()) {
        if temp_state.amount != params.amount {
            return Err(ProcessError::InvalidDeposit)?;
        }

        let transfer_ix = transfer(
            token_program.key,
            temp_account.key,
            stake_account.key,
            payer_account.key,
            &[&payer_account.key],
            params.amount as u64,
        )?;

        invoke(
            &transfer_ix,
            &[
                temp_account.clone(),
                stake_account.clone(),
                payer_account.clone(),
                token_program.clone(),
            ],
        )?;

        let close_temp_account_ix = close_account(
            token_program.key,
            temp_account.key,
            payer_account.key,
            payer_account.key,
            &[&payer_account.key],
        )?;

        invoke(
            &close_temp_account_ix,
            &[
                temp_account.clone(),
                payer_account.clone(),
                payer_account.clone(),
            ],
        )?;
    } else {
        // For native mint, just close the account
        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[game_account.key.as_ref()], program_id);

        if pda_account.key.ne(&pda) {
            return Err(ProcessError::InvalidPDA)?;
        }

        if temp_account.lamports() != params.amount {
            return Err(ProcessError::InvalidDeposit)?;
        }

        let close_temp_account_ix = close_account(
            token_program.key,
            temp_account.key,
            pda_account.key,
            payer_account.key,
            &[&payer_account.key],
        )?;

        invoke(
            &close_temp_account_ix,
            &[
                temp_account.clone(),
                pda_account.clone(),
                payer_account.clone(),
            ],
        )?;
    }

    GameState::pack(game_state, &mut game_account.try_borrow_mut_data()?)?;

    msg!(
        "Player {} joined the game {}",
        player_account.key,
        game_account.key
    );

    Ok(())
}
