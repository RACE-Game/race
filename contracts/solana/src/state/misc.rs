use crate::error::ProcessError;
use solana_program::{
    borsh::get_instance_packed_len, program_error::ProgramError, program_pack::Pack,
};

pub const PUBKEY_LEN: usize = 32;
pub const NAME_LEN: usize = 16;
pub const OPTION_PREFIX_LEN: usize = 1;
pub const COVER_LEN: usize = 43;
pub const DESC_LEN: usize = 140;
pub const REG_SIZE: usize = 100;
