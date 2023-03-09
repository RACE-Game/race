use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

// CreateGameAccountParams can be here or in solana.rs and should be the same as that in types
// It must be able to be serialized by borsh

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub title: String,
    pub bundle_addr: String,
    pub max_players: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CloseGameAccountParams {
    pub addr: String,
}


#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RegisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}


#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum RaceInstruction {
    /// #0 Create a new game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of transactor.
    /// 1. `[]` The transactor account.
    /// 2. `[writable]` The game account, hold all necessary info about the game.
    /// 3. `[writable]` The temp stake account.
    /// 4. `[]` The owner's account.
    /// 5. `[]` The mint account.
    /// 6. `[]` The scene NFT account.
    /// 7. `[]` The token program.
    CreateGameAccount { params: CreateGameAccountParams },

    /// #1 Create a new game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of transactor.
    /// 1. `[]` The transactor account.
    /// 2. `[writable]` The game account, hold all necessary info about the game.
    /// 3. `[writable]` The temp stake account.
    /// 4. `[]` The owner's account.
    /// 5. `[]` The mint account.
    /// 6. `[]` The scene NFT account.
    /// 7. `[]` The token program.
    CloseGameAccount { params: CloseGameAccountParams },

    /// #2 Register a game in lobby/center
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[]` The registration center account.
    /// 2. `[]` The account of game account.
    /// 3. `[writable]` The game reg account.
    RegisterGame { params: RegisterGameParams },
}

impl RaceInstruction {
    pub fn new(src: &[u8]) -> Self {
        RaceInstruction::try_from_slice(src).unwrap()
    }
}
