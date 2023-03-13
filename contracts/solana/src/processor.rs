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
        let instruction = RaceInstruction::unpack(instruction_data)?;
        match instruction {
            RaceInstruction::RegGame { params } => {
                msg!("Register Game Account on Chain");
                register_game::process(programe_id, accounts, params)
            }
            _ => Ok(())
        }
    }
}
