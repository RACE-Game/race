use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy)]
pub enum RaceError {
    /// 0
    UnpackOptionFailed,

    /// 1
    SeatIsOccupied,

    /// 2
    NoEnoughBalance,

    /// 3
    AmountOverflow,

    /// 4
    PlayerNotInGame,

    /// 5
    AlreadyInGame,

    /// 6
    InvalidGameLevel,

    /// 7
    InvalidGameType,

    /// 8
    InvalidGameStatus,

    /// 9
    GameIsLocked,

    /// a
    InvalidPlayerProfileAccount,

    /// b
    InvalidGameAccount,

    /// c
    InvalidBuyinAmount,

    /// d
    InvalidMint,

    /// e
    InvalidSettleUpdates,

    /// f
    InvalidAvatarPubkey,

    /// 10
    InvalidGameNo,

    /// 11
    InvalidTransactor,

    /// 12
    InvalidWinner,

    /// 13
    InvalidPlayerATA,

    /// 14
    CantRebuy,

    /// 15
    InvalidSettleSerial,

    /// 16
    InvalidTransactorRakeTaker,

    /// 17
    InvalidOwnerRakeTaker,

    /// 18
    AlreadyHasBonusAttached,

    /// 19
    InvalidBonusAccount,

    /// 1a
    InvalidBonusStakeAccount,

    /// 1b
    InvalidBonusPdaAccount,

    /// 1c
    InvalidBonusOwner,

    /// 1d
    InvalidOwner,

    /// 1e
    InvalidOwnerATA,

    /// 1f
    InvalidStakeAccount,

    /// 20
    InvalidPDA,

    /// 21
    CantCloseGame,

    /// 22
    InvalidRegistrationAccountSize,

    /// 23
    InvalidInstructionId,

    /// 24
    InvalidTournamentStatus,

    /// 25
    AlreadyRegistered,

    /// 26
    TournamentIsFull,

    /// 27
    InvalidRegistrationAccount,

    /// 28
    InvalidBlindsMode,

    /// 29
    InvalidQuota,

    /// 2a
    InvalidPrizeAccount,

    /// 2b
    AlreadyClaimed,

    /// 2c
    NoAvailablePrize,

    /// 2d
    MaximumNumberOfBonusesReached,

    /// 2e
    InvalidPlayerAccount,

    /// 2f
    InvalidTokenAccount,

    /// 30
    InvalidReceiverAccount,

    /// 31
    TournamentIsNotFreeroll,

    /// 32
    InvalidRegAccountSize,

    /// 33
    TournamentAlreadyRegistered,

    /// 34
    RegistrationIsFull,

    /// 35
    GameAlreadyRegistered,

    /// 36
    TournamentIsNotExpired,

    /// 37
    InvalidTournament,
}

impl From<RaceError> for ProgramError {
    fn from(e: RaceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
