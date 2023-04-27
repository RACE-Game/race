use solana_program::{program_error::ProgramError, msg};
use thiserror::Error;

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

    /// 8
    #[error("Expect writable player profile, found read-only")]
    InvalidAccountStatus,

    /// 9
    #[error("Account pubkey is not the same as that from transport")]
    InvalidAccountPubkey,

    /// A
    #[error("Settle amounts are not sum up to zero")]
    InvalidSettleAmounts,

    /// B
    #[error("Invalid settle player address")]
    InvalidSettlePlayerAddress,

    /// C
    #[error("Unhandled eliminated player")]
    UnhandledEliminatedPlayer,

    /// D
    #[error("Invalid receiver address, wallet and ATA mismatch")]
    InvalidRecevierAddress,

    /// E
    #[error("Settles are not in correct order")]
    InvalidOrderOfSettles,

    /// F
    #[error("Player balance amount overflows")]
    PlayerBalanceOverflow,

    /// 10
    #[error("Invalid voter account")]
    InvalidVoterAccount,

    /// 11
    #[error("Invalid votee account")]
    InvalidVoteeAccount,

    /// 12
    #[error("Game is not served")]
    GameNotServed,

    /// 13
    #[error("Feature is unimplemented")]
    Unimplemented,

    /// 14
    #[error("Duplicate joining not allowed as the server already joined")]
    DuplicateServerJoin,

    /// 15
    #[error("Can't unregister the game as it has not been registered yet")]
    InvalidUnregistration,

    /// 16
    #[error("Server number exceeds the max of 10")]
    ServerNumberExceedsLimit,

    /// 17
    #[error("Position already taken by another player")]
    PositionTakenAlready,

    /// 18
    #[error("Can't join game because game is already full")]
    GameFullAlready,

    /// 19
    #[error("Can't join game because player already joined")]
    JoinedGameAlready,

    /// 1A
    #[error("Token's mint must be the same as that used in the game")]
    InvalidMint,

    /// 1B
    #[error("Can't join game because deposit is invalid")]
    InvalidDeposit,

    /// 1C
    #[error("Given position falls out the range of 0 to player_num - 1")]
    InvalidPosition,
}

impl From<ProcessError> for ProgramError {
    fn from(err: ProcessError) -> Self {
        msg!("Processing error: {:?}", err);
        ProgramError::Custom(err as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_err_no() {
        assert_eq!(ProcessError::InvalidPosition as u32, 0x1C);
    }
}
