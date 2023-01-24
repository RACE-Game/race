use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

mod create_game_account;

use crate::instruction::RaceContractInstruction;

use crate::processor::create_game_account::process_create_game_account;
pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = RaceContractInstruction::try_from_slice(&instruction_data)?;

        let ret = match instruction {
            RaceContractInstruction::CreateGame(args) => {
                process_create_game_account(program_id, accounts, args)
            }
            RaceContractInstruction::CloseGame(_args) => todo!(),
        };

        if let Err(ref e) = ret {
            msg!("Error: {:?}", e);
        }
        ret
    }
}
