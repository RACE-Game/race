use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};
use race_solana_types::instruction::RaceInstruction;

mod create_game;
mod create_registry;
mod register_game;
mod close_game;

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = RaceInstruction::unpack(instruction_data)?;
    msg!("Instruction: {:?}", instruction);

    match instruction {
        RaceInstruction::CreateGameAccount { params } => {
            create_game::process(program_id, accounts, params)
        }

        RaceInstruction::CreateRegistry { params } => {
            msg!("Create a game center for registering games");
            create_registry::process(program_id, accounts, params)
        }

        RaceInstruction::RegGame { params } => {
            msg!("Register a game account on chain");
            register_game::process(program_id, accounts, params)
        }

        RaceInstruction::CloseGameAccount => {
            msg!("Close a game account on chain");
            close_game::process(program_id, accounts)
        }
    }
}
