use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct ServerParams {
    pub game_addr: String,
    pub server_addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub title: String,
    pub bundle_addr: String,
    pub max_players: u8,
    pub data: Vec<u8>,          // Data of a specific game type such as Holdem or Raffle
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CloseGameAccountParams {
    pub addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RegGameParams {
    pub game_addr: String,
    pub center_addr: String, // registration center address
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateRegistryParams {}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum RaceInstruction {
    /// #0 Create a new game
    ///
    /// TODO: pass bundle_addr as account
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

    /// #1 Create an on-chain "lobby" for game registration
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[writable]` The registry account.
    CreateRegistry { params: CreateRegistryParams },

    /// #2 Register a game in lobby/center
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[]` The registration center account.
    /// 2. `[]` The game account (extract all the important info)
    RegGame { params: RegGameParams },
}

impl RaceInstruction {
    pub fn unpack(src: &[u8]) -> Result<Self, ProgramError> {
        Ok(RaceInstruction::try_from_slice(src).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser() -> anyhow::Result<()> {
        let ix = RaceInstruction::CreateGameAccount{params: CreateGameAccountParams{ title: "Holdem".to_string(), bundle_addr: "JJJJJJ".to_string(), max_players: 8, data: vec![] }};
        let data = ix.try_to_vec()?;
        println!("data: {:?}", data);
        assert_eq!(1, 2);
        Ok(())
    }
}
