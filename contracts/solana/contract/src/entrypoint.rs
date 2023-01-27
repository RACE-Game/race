use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, log::sol_log_compute_units, msg,
    pubkey::Pubkey,
};

use crate::processor::Processor;

solana_program::entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("KS:LDKSA");
    let ret = Processor::process(program_id, accounts, instruction_data);
    sol_log_compute_units();
    ret
}
