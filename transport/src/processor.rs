use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

mod create_game;
mod register_game;

use crate::instruction::{RaceInstruction, self};

pub struct Processor {}

impl Processor {
    pub fn process(
        programe_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8]
    ) -> ProgramResult {
        let instruction = RaceInstruction::try_from_slice(instruction_data)?;
        match instruction {
            RaceInstruction::CreateGameAccount { params } => {
                msg!("Creating Game Account on Chain");
                create_game::process(programe_id, accounts, instruction_data)
            }
            _ => Ok(())
        }
    }
}
