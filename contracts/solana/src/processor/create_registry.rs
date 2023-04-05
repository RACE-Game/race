use crate::state::{GameReg, RegistryState};
use race_solana_types::types::CreateRegistrationParams;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar, msg,
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

    // let mut registry = RegistryState::unpack_unchecked(&registry_account.try_borrow_data()?)?;
    //
    // if registry.is_initialized {
    //     return Err(ProgramError::AccountAlreadyInitialized);
    // }

    let mut registry_state = RegistryState {
        is_initialized: true,
        is_private: params.is_private,
        addr: registry_account.key.clone(),
        size: params.size,
        owner: payer.key.clone(),
        games: Default::default(),
        // games: Box::new(Vec::<GameReg>::with_capacity(params.size as usize)),
        padding: Default::default(),
    };

    registry.is_initialized = true;
    registry.owner = payer.key.clone();
    registry_state.update_padding();

    RegistryState::pack(registry_state, &mut registry_account.try_borrow_mut_data()?)?;
    msg!("Created registry {}", registry_account.key);

    Ok(())
}
