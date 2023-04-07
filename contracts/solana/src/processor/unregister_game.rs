// use arrayref::array_mut_ref;
use crate::{
    error::ProcessError,
    state::{GameState, RegistryState, Padded},
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

#[inline(never)]
pub fn process(
    _programe_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let registry_account = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("1");
    let mut registry_state = RegistryState::unpack(&registry_account.try_borrow_data()?)?;
    if !registry_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    msg!("2");

    let rent = Rent::get()?;
    if !rent.is_exempt(registry_account.lamports(), RegistryState::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }
    msg!("3");

    if registry_state.is_private && registry_state.owner.ne(payer.key) {
        return Err(ProcessError::InvalidOwner)?;
    }
    msg!("4");

    let game_state = GameState::unpack(&game_account.try_borrow_data()?)?;
    if !game_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    if game_state.owner.ne(payer.key) {
        return Err(ProcessError::InvalidOwner)?;
    }
    msg!("6");

    let mut removed = false;
    if registry_state.games.iter().find(|reg| reg.addr.eq(game_account.key)).is_none() {
        msg!("0");
        return Err(ProcessError::InvalidUnregistration)?;
    } else if !removed {
        let mut unreg_idx = 0usize;
        for (idx, game) in registry_state.games.iter().enumerate()  {
            if game.addr.eq(game_account.key) {
                unreg_idx = idx;
                break;
            }
        }
        let unreg_game = registry_state.games.remove(unreg_idx);
        msg!("Unregitered game {}", unreg_game.addr);

        registry_state.update_padding()?;
        RegistryState::pack(registry_state, &mut registry_account.try_borrow_mut_data()?)?;

        removed = true;
        msg!("7");
    }

    if !removed {
        return Err(ProcessError::InvalidUnregistration)?;
    }

    Ok(())
}
