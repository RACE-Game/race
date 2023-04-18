use crate::state::{GameReg, Padded, RegistryState};
use race_solana_types::types::CreateRegistrationParams;
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
    params: CreateRegistrationParams,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let registry_account = next_account_info(account_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let rent = Rent::get()?;
    if !rent.is_exempt(registry_account.lamports(), RegistryState::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    let mut registry_state = RegistryState {
        is_initialized: true,
        is_private: params.is_private,
        size: params.size,
        owner: payer.key.clone(),
        games: Box::new(Vec::<GameReg>::with_capacity(params.size as usize)),
        padding: Default::default(),
    };

    registry_state.update_padding()?;

    RegistryState::pack(registry_state, &mut registry_account.try_borrow_mut_data()?)?;
    msg!("Created registry {}", registry_account.key);

    Ok(())
}
