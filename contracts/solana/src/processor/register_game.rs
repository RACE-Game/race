// use arrayref::array_mut_ref;
use crate::{
    error::RaceError,
    state::{GameReg, GameState, RegistryState},
};
use borsh::{BorshDeserialize, BorshSerialize};
use race_solana_types::types::RegisterGameParams;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
};
use std::time::{SystemTime, SystemTimeError};

#[inline(never)]
pub fn process(
    _programe_id: &Pubkey,
    accounts: &[AccountInfo],
    _params: RegisterGameParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let reg_center_account = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;
    // let reg_game_account = next_account_info(account_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut reg_center_state = RegistryState::unpack(&reg_center_account.try_borrow_data()?)?;
    if !reg_center_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    let rent = Rent::default();
    if !rent.is_exempt(reg_center_account.lamports(), RegistryState::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    if reg_center_state.is_private && reg_center_state.owner.ne(payer.key) {
        // TODO: Improve the error
        return Err(RaceError::InvalidOwner)?;
    }
    if reg_center_state.games.len() as u16 == reg_center_state.size {
        // TODO: Implement error "Registration center is already full"
        return Err(ProgramError::Custom(1));
    }

    let game_state = GameState::unpack(&game_account.try_borrow_data()?)?;
    if game_state.owner.ne(payer.key) {
        return Err(RaceError::InvalidOwner)?;
    }

    let mut added = false;
    if let Some(reg_game) = reg_center_state
        .games
        .iter()
        .find(|gr| gr.addr.eq(&game_account.key))
    {
        return Err(ProgramError::Custom(2));
    } else if !added {
        added = true;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Timestamp")
            .as_secs() as u64;
        let reg_game = GameReg {
            title: "RACE DREAM".to_string(),
            addr: game_account.key.clone(),
            reg_time: timestamp,
            // mint: game_state.mint_pubkey.clone(),
            // is_hidden: params.is_hidden,
        };

        reg_center_state.games.push(reg_game);
        reg_center_state.update_padding();
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(reg_center_state, &mut buf)?;
    }

    if !added {
        return Err(RaceError::RegistrationIsFull)?;
    }

    Ok(())
}
