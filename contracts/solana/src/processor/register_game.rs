use arrayref::array_mut_ref;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process(
    programe_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let owner_account = next_account_info(account_iter)?;
    let reg_center_account = next_account_info(account_iter)?;
    let game_account = next_account_info(account_iter)?;
    let game_reg_account = next_account_info(account_iter)?;

 if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let game_state = GameState::unpack(&game_account.try_borrow_data()?)?;

    if game_state.owner_pubkey.ne(owner_account.key) {
        return Err(DealerError::InvalidOwner)?;
    }

    let reg_center_state = RegCenter::unpack(&reg_center_account.try_borrow_data()?)?;

    if reg_center_state.is_private && reg_center_state.owner.ne(owner_account.key) {
        return Err(DealerError::InvalidOwner)?;
    }

    let mut reg_data = game_reg_account.try_borrow_mut_data()?;

    let mut added = false;

    for i in 0..100 {
        let reg_src = array_mut_ref![reg_data, i * GameReg::OPT_LEN, GameReg::OPT_LEN];
        let reg = GameReg::unpack_option(reg_src)?;
        if let Some(reg) = reg {
            if reg.pubkey.eq(game_account.key) {
                return Err(DealerError::GameAlreadyRegistered)?;
            }
        } else if !added {
            added = true;

            let reg_state = GameReg {
                pubkey: game_account.key.clone(),
                mint: game_state.mint_pubkey.clone(),
                is_hidden: params.is_hidden,
            };

            GameReg::pack_option(Some(reg_state), reg_src)?;

            break;
        }
    }

    if !added {
        return Err(DealerError::RegistrationIsFull)?;
    }

    Ok(())
}
