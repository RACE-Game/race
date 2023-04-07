use crate::{
    error::ProcessError,
    state::{GameReg, GameState, RegistryState, Padded},
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use std::time::SystemTime;

#[inline(never)]
pub fn process(
    _programe_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let registry_account = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;

    msg!("payer pubkey {}", payer.key.clone());
    msg!("reg account {}", registry_account.key.clone());

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut registry_state = RegistryState::unpack(&registry_account.try_borrow_data()?)?;
    msg!("owner pubkey {}", registry_state.owner.clone());

    if !registry_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    let rent = Rent::get()?;
    if !rent.is_exempt(registry_account.lamports(), RegistryState::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    if registry_state.is_private && registry_state.owner.ne(payer.key) {
        return Err(ProcessError::InvalidOwner)?;
    }

    // TODO: Check on transport side?
    if registry_state.games.len() as u16 == registry_state.size {
        return Err(ProcessError::RegistrationIsFull)?;
    }

    let game_state = GameState::unpack(&game_account.try_borrow_data()?)?;
    if !game_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    if game_state.owner.ne(payer.key) {
        return Err(ProcessError::InvalidOwner)?;
    }

    let mut added = false;
    if registry_state.games.len() > 0 {
        if registry_state.games.iter().any(|reg| reg.addr.eq(game_account.key)) {
            msg!("0");
            return Err(ProcessError::GameAlreadyRegistered)?;
        }
    }
    else if registry_state.games.len() == 0 {
        // FIXME
        let timestamp = 1680971620u64;
        // let timestamp = SystemTime::now()
        //     .duration_since(SystemTime::UNIX_EPOCH)
        //     .expect("Timestamp")
        //     .as_secs() as u64;
        let reg_game = GameReg {
            title: game_state.title.clone(),
            addr: game_account.key.clone(),
            reg_time: timestamp,
            bundle_addr: game_state.bundle_addr.clone(),
            // is_hidden: params.is_hidden,
        };

        registry_state.games.push(reg_game);
        registry_state.update_padding()?;

        RegistryState::pack(registry_state, &mut registry_account.try_borrow_mut_data()?)?;

        added = true;
        msg!(
            "Registered game {} to {}",
            game_account.key.clone(),
            registry_account.key.clone()
        );
    }

    if !added {
        return Err(ProcessError::RegistrationIsFull)?;
    }

    Ok(())
}
