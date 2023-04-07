use crate::types::{
    CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams,
    RegisterServerParams, SettleParams, VoteParams,
};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::program_error::ProgramError;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum RaceInstruction {
    /// # Create a new game
    ///
    /// TODO: pass bundle_addr as account
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of transactor.
    /// 1. `[writable]` The game account, hold all necessary info about the game.
    /// 2. `[writable]` The temp stake account.
    /// 3. `[]` The mint account.
    /// 4. `[]` The scene NFT account.
    /// 5. `[]` The bundled data account
    // TODO: add Game scene NFT to this ix
    CreateGameAccount { params: CreateGameAccountParams },

    /// # Close a new game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[]` The account of game account.
    /// 2. `[writable]` The game reg account.
    /// 3. `[writable]` The stake account of game.
    /// 4. `[]` PDA account.
    /// 5. `[]` Token program.
    // TODO: add registration center account to this ix?
    CloseGameAccount,

    /// # Create an on-chain "lobby" for game registration
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[writable]` The registry account.
    CreateRegistry { params: CreateRegistrationParams },

    /// # Create a player profile
    ///
    /// Accounts expected:
    /// 0. `[signer]` The owner of the player profile
    /// 1. `[]` The player profile account to be created
    /// 2. `[]` The pfp account
    CreatePlayerProfile { params: CreatePlayerProfileParams },

    /// # Register (Create) a server profile
    ///
    /// Accounts expected:
    /// 0. `[signer]` The owner of the player profile
    /// 1. `[]` The server profile account to be created
    RegisterServer { params: RegisterServerParams },

    /// # Settle game result
    ///
    /// Accounts expected:
    /// 0. `[signer]` The game transactor account
    /// 1. `[writable]` The game account
    /// 2. `[writable]` The stake account, must match the one in game account
    /// 3. `[]` PDA account
    /// 4. `[]` The token program
    /// 5. `[]` The system program
    /// Following:
    /// `[]` Every leaving players account, must be in the same order with Eject settles
    Settle { params: SettleParams },

    /// # Vote
    ///
    /// Accounts expected:
    /// 0. `[signer]` The voter account, could be the wallet address of a server or a player.
    /// 1. `[writable]` The game account.
    /// 2. `[]` The votee account.
    Vote { params: VoteParams },

    /// # Serve a game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer acount (the server itself)
    /// 1. `[writable]` The game account to be served
    /// 2. `[]` The server account
    ServeGame,

    /// # Register a game to the registry
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer acount (game account onwer?)
    /// 1. `[writable]` The registry account
    /// 2. `[]` The game account to be registered
    RegisterGame,

    /// # Unregister a game to the registry
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer acount (game account onwer?)
    /// 1. `[writable]` The registry account
    /// 2. `[]` The game account to be unregistered
    UnregisterGame,
}

#[cfg(feature = "program")]
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
        let ix = RaceInstruction::CreateGameAccount {
            params: CreateGameAccountParams {
                title: "Holdem".to_string(),
                max_players: 8,
                data: vec![],
            },
        };
        let data = ix.try_to_vec()?;
        println!("data: {:?}", data);
        assert_eq!(1, 2);
        Ok(())
    }
}
