use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

// mod create_game;
// mod register_game;
mod create_registry;

use crate::instruction::RaceInstruction;

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = RaceInstruction::unpack(instruction_data)?;
    msg!("Instruction: {:?}", instruction);
    match instruction {
        // RaceInstruction::RegGame { params } => {
        //     msg!("Register Game Account on Chain");
        //     register_game::process(programe_id, accounts, params)
        // }
        RaceInstruction::CreateRegistry { params } => {
            create_registry::process(program_id, accounts, params)
        }
        _ => Ok(()),
    }
}