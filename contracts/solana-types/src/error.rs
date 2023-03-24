use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug)]
pub enum InstructionError {
    #[error("failed to initialize the account")]
    InitAccountFailed,

    #[error("failed to initialize the instruction: {0}")]
    InitInstructionFailed(String),

    #[error("failed to pack the instruction data")]
    PackingInstructionDataError,
}

#[derive(Error, Debug)]
pub enum ContractError {
    /// 0
    #[error("failed to create game")]
    CreateGameFailed,
    // 1

    // 2
}

impl From<ContractError> for ProgramError {
    fn from(err: ContractError) -> Self {
        ProgramError::Custom(err as u32)
    }
}

// pub type InstructionResult<T> = std::result::Result<T, InstructionError>;

// impl From<InstructionError> for race_core::error::Error {
//     fn from(error: InstructionError) -> Self {
//         Self::InitInstructionFailed(error.to_string())
//     }
// }
