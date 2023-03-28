use std::str::FromStr;

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    instruction::{close_account, transfer},
    state::Account,
};

use crate::error::ProcessError;

const NATIVE_MINT: &str = "So11111111111111111111111111111111111111112";

pub fn is_native_mint(mint: &Pubkey) -> bool {
    mint.eq(&Pubkey::from_str(NATIVE_MINT).unwrap())
}

pub fn validate_receiver_account(
    account: &Pubkey,
    mint: &Pubkey,
    receiver: &Pubkey,
) -> ProgramResult {
    if is_native_mint(mint) {
        if receiver.ne(account) {
            msg!(
                "Invalid receiver, expected: {:?}, actual: {:?}",
                account,
                receiver
            );
            return Err(ProcessError::InvalidRecevierAddress)?;
        }
    } else {
        let ata = get_associated_token_address(account, mint);
        if receiver.ne(&ata) {
            msg!(
                "Invalid receiver, expected: {:?}, actual: {:?}",
                ata,
                receiver
            );
            return Err(ProcessError::InvalidRecevierAddress)?;
        }
    }
    Ok(())
}

pub struct TempSource<'a> {
    token_program: AccountInfo<'a>,
    temp_account: AccountInfo<'a>,
    provider_account: AccountInfo<'a>,
}

impl<'a> TempSource<'a> {
    pub fn new(
        provider_account: AccountInfo<'a>,
        temp_account: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
    ) -> Self {
        Self {
            provider_account,
            temp_account,
            token_program,
        }
    }

    pub fn transfer(&self, dest: &AccountInfo<'a>, amount: u64) -> ProgramResult {
        let transfer_ix = transfer(
            self.token_program.key,
            self.temp_account.key,
            dest.key,
            self.provider_account.key,
            &[&self.provider_account.key],
            amount,
        )?;
        invoke(
            &transfer_ix,
            &[
                self.temp_account.clone(),
                dest.clone(),
                self.provider_account.clone(),
                self.token_program.clone(),
            ],
        )?;
        Ok(())
    }
    pub fn close(&self) -> ProgramResult {
        let close_ticket_asset_account_ix = close_account(
            self.token_program.key,
            self.temp_account.key,
            self.provider_account.key,
            self.provider_account.key,
            &[&self.provider_account.key],
        )?;

        invoke(
            &close_ticket_asset_account_ix,
            &[
                self.temp_account.clone(),
                self.provider_account.clone(),
                self.provider_account.clone(),
            ],
        )?;

        Ok(())
    }
}

/// Wrap a token account into a transfer source for easy token
/// transfer.  Support both WSOL and other SPL tokens.
pub struct TransferSource<'a> {
    pub system_program: AccountInfo<'a>,
    pub token_program: AccountInfo<'a>,
    pub source_account: AccountInfo<'a>,
    pub program_id: Pubkey,
    pub pda_seed: &'a [u8],
    pub pda_account: AccountInfo<'a>,
    pub bump_seed: u8,
    pub is_native_mint: bool,
}

impl<'a> TransferSource<'a> {
    #[inline(never)]
    pub fn try_new(
        system_program: AccountInfo<'a>,
        token_program: AccountInfo<'a>,
        source_account: AccountInfo<'a>,
        pda_seed: &'a [u8],
        pda_account: AccountInfo<'a>,
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        let source_state = Account::unpack(&source_account.try_borrow_data()?)?;
        let (pda_pubkey, bump_seed) = Pubkey::find_program_address(&[pda_seed], program_id);
        if pda_account.key.ne(&pda_pubkey) {
            return Err(ProcessError::InvalidPDA)?;
        }
        Ok(Self {
            system_program,
            token_program,
            source_account,
            pda_seed,
            program_id: program_id.clone(),
            pda_account,
            bump_seed,
            is_native_mint: is_native_mint(&source_state.mint),
        })
    }

    #[inline(never)]
    pub fn transfer(&self, dest: &AccountInfo<'a>, amount: u64) -> ProgramResult {
        if self.is_native_mint {
            self.transfer_sol(dest, amount)?;
        } else {
            self.transfer_token(dest, amount)?;
        }
        Ok(())
    }

    /// Transfer SOL tokens from src(WSOL token account) to dest(wallet account).
    #[inline(never)]
    fn transfer_sol(&self, dest: &AccountInfo<'a>, amount: u64) -> ProgramResult {
        let ix =
            solana_program::system_instruction::transfer(self.pda_account.key, dest.key, amount);

        invoke_signed(
            &ix,
            &[self.pda_account.clone(), dest.clone()],
            &[&[self.pda_seed, &[self.bump_seed]]],
        )?;

        Ok(())
    }

    /// Transfer SPL tokens from src(token account) to dest(token account).
    #[inline(never)]
    fn transfer_token(
        &self,
        dest: &AccountInfo<'a>,
        amount: u64,
    ) -> ProgramResult {
        msg!("Send {:?} SPL to {:?}", amount, dest.key);

        if Account::unpack(&dest.try_borrow_data()?).is_ok() {
            let ix = transfer(
                self.token_program.key,
                self.source_account.key,
                dest.key,
                &self.pda_account.key,
                &[&self.pda_account.key],
                amount,
            )?;

            invoke_signed(
                &ix,
                &[
                    self.source_account.clone(),
                    dest.clone(),
                    self.pda_account.clone(),
                ],
                &[&[self.pda_seed, &[self.bump_seed]]],
            )?;
        } else {
            msg!("Receiver account {:?} not available", dest.key);
        }

        Ok(())
    }
}
