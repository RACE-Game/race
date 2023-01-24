use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, GameAccount, PlayerDeposit, PlayerJoin,
};
use solana_program::program::invoke_signed;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};
use std::mem;
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

#[derive(Default, BorshSerialize, BorshDeserialize, Clone)]
pub struct UserAccount {
    pub post_count: u128,
    pub follower_count: u128,
    pub following_count: u128,
}

impl UserAccount {
    pub fn space() -> usize {
        mem::size_of::<UserAccount>()
    }
    pub fn seed(address: Pubkey) -> Vec<u8> {
        let res = format!("account-{address}");
        solana_program::hash::hash(res.as_bytes())
            .to_bytes()
            .to_vec()
    }
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

pub fn execute_instruction<'a>(
    program_id: &Pubkey,
    creator: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    data: &[u8],
) -> ProgramResult {
    // let mut user_account_state = if self.accounts.account_state.data.borrow().len() > 0 {
    //     UserAccount::deserialize(&mut &**self.accounts.account_state.data.borrow())?
    // } else {

    create_pda(
        program_id,
        UserAccount::space(),
        creator,
        pda,
        system_program,
        UserAccount::seed(*creator.key).as_slice(),
    )?;

    let user_account_state = UserAccount::try_from_slice(&data)?;
    // let user_account_state = UserAccount::default();
    // };
    // user_account_state.profile = self.args.profile.clone();
    user_account_state.serialize(&mut &mut pda.data.borrow_mut()[..])?;

    Ok(())
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
// impl GameAccount {
//     pub fn space() -> usize {
//         mem::size_of::<UserAccount>()
//     }
//     pub fn seed(address: Pubkey) -> Vec<u8> {
//         let res = format!("account-{address}");
//         solana_program::hash::hash(res.as_bytes())
//             .to_bytes()
//             .to_vec()
//     }
// }
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
        UserAccount::seed(*signer.key).as_slice(),
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
    // msg!("HERE1");
    // let account_info_iter = &mut accounts.iter();

    // let account_state = next_account_info(account_info_iter)?;
    // let system_program = next_account_info(account_info_iter)?;
    // let signer = next_account_info(account_info_iter)?;

    // // let user = UserAccount {
    // //     post_count: 0,
    // //     follower_count: 0,
    // //     following_count: 0,
    // // };
    // // user.serialize(&mut *account_state.try_borrow_mut_data()?)?;
    // execute_instruction(
    //     program_id,
    //     signer,
    //     account_state,
    //     system_program,
    //     instruction_data,
    // );
    // msg!("HERE2");
    // return Ok(());
    // msg!("HERE3");
    // let payer = next_account_info(account_info_iter)?;
    // // The vault PDA, derived from the payer's address
    // let vault = next_account_info(account_info_iter)?;

    // let lamports = 10000;
    // let vault_size = 1;

    // // Invoke the system program to create an account while virtually
    // // signing with the vault PDA, which is owned by this caller program.

    // let (vault_pubkey, vault_bump_seed) =
    //     Pubkey::find_program_address(&[b"vault", payer.key.as_ref()], &program_id);
    // msg!("BUBUB {} {} {}", vault_pubkey, &vault.key, vault_bump_seed);
    // msg!("Invoking 1");
    // let res = invoke_signed(
    //     &solana_program::system_instruction::create_account(
    //         &payer.key,
    //         &vault.key,
    //         lamports,
    //         vault_size,
    //         &program_id,
    //     ),
    //     &[payer.clone(), vault.clone()],
    //     // A slice of seed slices, each seed slice being the set
    //     // of seeds used to generate one of the PDAs required by the
    //     // callee program, the final seed being a single-element slice
    //     // containing the `u8` bump seed.
    //     &[&[&payer.key.as_ref(), &[vault_bump_seed]]],
    // );
    // msg!("Invoking 2");

    let instruction = Instruction::try_from_slice(&instruction_data)?;

    match instruction {
        Instruction::CreateGame(args) => process_create_game(program_id, accounts, args)?,
        Instruction::CloseGame(args) => close_game(args),
    };

    // msg!("Our program's Program ID: {}", &program_id);

    // let accounts_iter = &mut accounts.iter();
    // let payer = next_account_info(accounts_iter)?;

    // msg!("Payer Address: {}", payer.key);

    Ok(())
}
