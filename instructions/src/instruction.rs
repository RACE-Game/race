use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use race_core::types::{
    CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, GameRegistration,
    PlayerJoin, RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams,
    ServerAccount, ServerJoin, UnregisterGameParams, VoteParams,
};


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
    CreateRegistry { params: CreateRegistrationParams },

    /// #2 Register a game in lobby/center
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[]` The registration center account.
    /// 2. `[]` The game account (extract all the important info)
    RegGame { params: RegisterGameParams },
}

impl RaceInstruction {
    pub fn pack(instruction: RaceInstruction) -> Result<Vec<u8>, ProgramError> {
        Ok(instruction.try_to_vec()?)
    }

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
