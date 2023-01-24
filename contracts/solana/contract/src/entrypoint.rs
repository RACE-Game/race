use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, GameAccount, PlayerDeposit, PlayerJoin,
};
use solana_program::program::invoke_signed;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};
use std::mem;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    CreateGame(CreateGameAccountParams),
    CloseGame(CloseGameAccountParams),
}

// https://github.com/shravanshetty1/blawgd-solana/blob/908f80a69050feec7b6cd53631813b316f54e1cc/blawgd-solana-sc/src/util.rs
pub fn create_pda<'a>(
    program_id: &Pubkey,
    space: usize,
    creator: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    seed: &[u8],
) -> ProgramResult {
    let rent = solana_program::sysvar::rent::Rent::get()?.minimum_balance(space);

    let ix = solana_program::system_instruction::create_account(
        creator.key,
        pda.key,
        rent,
        space as u64,
        program_id,
    );

    // TODO get nonce from args?

    let (_, nonce) = Pubkey::find_program_address(&[seed], program_id);
    let result = invoke_signed(
        &ix,
        &[creator.clone(), pda.clone(), system_program.clone()],
        &[&[seed, &[nonce]]],
    );
    result
}

fn game_account_max_space(params: &CreateGameAccountParams) -> usize {
    let mut result = 0;
    result += params.bundle_addr.len() * 8; // pub addr: String
    result += params.bundle_addr.len() * 8; // pub bundle_addr: String
    result += mem::size_of::<u64>(); // pub settle_version: u64
    result += mem::size_of::<u64>(); // pub access_version: u64

    result += (params.max_players as usize) * mem::size_of::<PlayerJoin>(); // pub players: Vec<PlayerJoin>
    result += (params.max_players as usize) * mem::size_of::<PlayerDeposit>(); // pub players: Vec<PlayerDeposit>
                                                                               // TODO: handle (server_addrs)
    result += 0; // pub server_addrs: Vec<String>
    result += params.bundle_addr.len() * 8; // pub transactor_addr: Option<String>
    result += mem::size_of::<u8>(); // pub max_players: u8
    result += mem::size_of::<u32>(); // pub data_ln: u32
    result += params.data.len() * mem::size_of::<u8>(); // pub data_ln: u32
    result
}

pub fn game_account_seed(address: Pubkey) -> Vec<u8> {
    let res = format!("game_account-{address}");
    solana_program::hash::hash(res.as_bytes())
        .to_bytes()
        .to_vec()
}

fn process_create_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateGameAccountParams,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let account_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let signer = next_account_info(account_info_iter)?;

    create_pda(
        program_id,
        game_account_max_space(&args),
        signer,
        account_state,
        system_program,
        game_account_seed(*signer.key).as_slice(),
    )?;

    let mut account = GameAccount::default();
    account.max_players = args.max_players;
    account.addr = account_state.key.to_string();
    account.bundle_addr = args.bundle_addr;
    account.data_len = args.data.len() as u32;
    account.data = args.data;
    account.serialize(&mut &mut account_state.data.borrow_mut()[..])?;
    Ok(())
}
solana_program::entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(&instruction_data)?;

    match instruction {
        Instruction::CreateGame(args) => process_create_game(program_id, accounts, args)?,
        Instruction::CloseGame(_args) => todo!(),
    };

    Ok(())
}
