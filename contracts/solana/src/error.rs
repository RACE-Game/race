use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug)]
pub enum ProcessError {
    /// 0
    #[error("invalid owner of this account")]
    InvalidOwner,

    /// 1
    #[error("failed to create game")]
    CreateGameFailed,

    /// 2
    #[error("registration center is already full")]
    RegistrationIsFull,

    /// 3
    #[error("game already registered")]
    GameAlreadyRegistered,

    /// 4
    #[error("unable to close game that still has players in it")]
    CantCloseGame,

    /// 5
    #[error("invalid stake account")]
    InvalidStakeAccount,

    /// 6
    #[error("invalid program derived address")]
    InvalidPDA,

    /// 7
    #[error("Account stake amount overflows")]
    StakeAmountOverflow,
}

impl From<ProcessError> for ProgramError {
    fn from(err: ProcessError) -> Self {
        ProgramError::Custom(err as u32)
    }
}
