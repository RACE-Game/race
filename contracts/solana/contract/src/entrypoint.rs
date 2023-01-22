use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{CloseGameAccountParams, CreateGameAccountParams};
use solana_program::program::invoke_signed;
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

#[derive(BorshSerialize, BorshDeserialize)]
pub struct InstructionData {
    pub vault_bump_seed: u8,
    pub lamports: u64,
}

fn create_game(program_id: &Pubkey, args: CreateGameAccountParams) {
    msg!(
        "Processing create_game {} {}",
        args.max_players,
        args.bundle_addr
    );

    let (pda, bump_seed) = Pubkey::find_program_address(&[b"test"], &program_id);
    let derived_pubkey = Pubkey::create_with_seed(&program_id, "test", &program_id).unwrap();
    msg!("pda: {}, bump: {}", pda, bump_seed);
    msg!("account pubkey: {:?}", derived_pubkey);
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
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    // The vault PDA, derived from the payer's address
    let vault = next_account_info(account_info_iter)?;

    let lamports = 10000;
    let vault_size = 1;

    // Invoke the system program to create an account while virtually
    // signing with the vault PDA, which is owned by this caller program.

    let (vault_pubkey, vault_bump_seed) =
        Pubkey::find_program_address(&[b"vault", payer.key.as_ref()], &program_id);
    msg!("BUBUB {} {} {}", vault_pubkey, &vault.key, vault_bump_seed);
    msg!("Invoking 1");
    let res = invoke_signed(
        &solana_program::system_instruction::create_account(
            &payer.key,
            &vault.key,
            lamports,
            vault_size,
            &program_id,
        ),
        &[payer.clone(), vault.clone()],
        // A slice of seed slices, each seed slice being the set
        // of seeds used to generate one of the PDAs required by the
        // callee program, the final seed being a single-element slice
        // containing the `u8` bump seed.
        &[&[&payer.key.as_ref(), &[vault_bump_seed]]],
    );
    msg!("Invoking 2");

    let instruction = Instruction::try_from_slice(&instruction_data)?;

    match instruction {
        Instruction::CreateGame(args) => create_game(program_id, args),
        Instruction::CloseGame(args) => close_game(args),
    };

    msg!("Our program's Program ID: {}", &program_id);

    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    msg!("Payer Address: {}", payer.key);

    Ok(())
}
