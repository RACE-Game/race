use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};
use race_solana_types::instruction::RaceInstruction;

mod create_game;
mod create_registry;
mod create_profile;
mod register_game;
mod close_game;
mod register_server;
mod settle;
mod misc;

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
        RaceInstruction::CreatePlayerProfile { params } => {
            msg!("Create a player profile on chain");
            create_profile::process(program_id, accounts, params)
        }
        RaceInstruction::RegisterServer { params } => {
            msg!("Create a server account on chain");
            register_server::process(program_id, accounts, params)
        }
        RaceInstruction::Settle { params } => {
            msg!("Settle game");
            settle::process(program_id, accounts, params)
        }
    }
}
