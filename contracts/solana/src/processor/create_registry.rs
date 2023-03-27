use crate::state::RegistryState;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use race_solana_types::types::CreateRegistrationParams;

#[inline(never)]
pub fn process(
    _programe_id: &Pubkey,
    accounts: &[AccountInfo],
    _params: CreateRegistrationParams,
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

    let mut registry = RegistryState::unpack_unchecked(&registry_account.try_borrow_data()?)?;

    if registry.is_initialized {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    registry.is_initialized = true;
    registry.owner = payer.key.clone();

    registry.update_padding();

    RegistryState::pack(registry, &mut registry_account.try_borrow_mut_data()?)?;

    Ok(())
}
