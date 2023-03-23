use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstructionError {
    #[error("Failed to Initialize the account")]
    InitAccountFailed,

    #[error("Failed to Initialize the instruction: {0}")]
    InitInstructionFailed(String),

    #[error("Failed to pack the instruction data")]
    PackingInstructionDataError,
}

pub type InstructionResult<T> = std::result::Result<T, InstructionError>;

// impl From<InstructionError> for race_core::error::Error {
//     fn from(error: InstructionError) -> Self {
//         Self::InitInstructionFailed(error.to_string())
//     }
// }
