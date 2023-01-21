use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{CloseGameAccountParams, CreateGameAccountParams};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    CreateGame(CreateGameAccountParams),
    CloseGame(CloseGameAccountParams),
}

fn create_game(args: CreateGameAccountParams) {
    msg!(
        "Processing create_game {} {}",
        args.max_players,
        args.bundle_addr
    );
}

fn close_game(args: CloseGameAccountParams) {
    msg!("Processing close_game {}", args.addr);
}

solana_program::entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(&instruction_data)?;

    match instruction {
        Instruction::CreateGame(args) => create_game(args),
        Instruction::CloseGame(args) => close_game(args),
    };

    msg!("Our program's Program ID: {}", &program_id);

    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    msg!("Payer Address: {}", payer.key);

    Ok(())
}
