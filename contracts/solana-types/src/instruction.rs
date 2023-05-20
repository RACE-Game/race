use crate::types::{
    CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, JoinParams,
    PublishParams, RegisterServerParams, SettleParams, VoteParams, ServeParams,
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
    /// 4. `[]` The token program.
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
    ServeGame{ params: ServeParams },

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

    /// # Join a game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The player to join the game
    /// 1.
    /// 1. `[writable]` The temp account.
    /// 2. `[writable]` The game account
    /// 3. `[]` The mint account.
    /// 4. `[writable]` The stake account that holds players' buyin assets
    /// 5. `[writable]` The pda account
    /// 6. `[]` The SPL token program
    JoinGame { params: JoinParams },

    /// # Publish a game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer account
    /// 1. `[]` The mint account.
    /// 2. `[writable]` The ata account.
    /// 3. `[]` The metadata PDA.
    /// 4. `[]` The edition PDA.
    /// 5. `[]` The token program.
    /// 6. `[]` The metaplex program.
    /// 7. `[]` The sys rent program.
    /// 8. `[]` The system program.
    PublishGame { params: PublishParams },
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
    fn test_ser_create_game_account() -> anyhow::Result<()> {
        let nodata_ix = RaceInstruction::CreateGameAccount{
            params: CreateGameAccountParams {
                title: "test game".to_string(),
                min_deposit: 30u64,
                max_deposit: 60u64,
                max_players: 10u16,
                data: vec![]
            }
        };

        let data_ix = RaceInstruction::CreateGameAccount{
            params: CreateGameAccountParams {
                title: "test game #2".to_string(),
                min_deposit: 10u64,
                max_deposit: 20u64,
                max_players: 10u16,
                data: vec![1, 2, 3, 4],
            }
        };

        let nodata_ix_ser = nodata_ix.try_to_vec().unwrap();
        println!("No data ix {:?}", nodata_ix_ser);
        let nodata_bytes = [0, 9, 0, 0, 0, 116, 101, 115, 116, 32, 103, 97, 109, 101, 10, 0, 30, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(nodata_ix_ser, nodata_bytes);

        let data_ix_ser = data_ix.try_to_vec().unwrap();
        println!("Data ix {:?}", data_ix_ser);
        let data_bytes = [0, 12, 0, 0, 0, 116, 101, 115, 116, 32, 103, 97, 109, 101, 32, 35, 50, 10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 1, 2, 3, 4];
        assert_eq!(data_ix_ser, data_bytes);

        Ok(())
    }

    #[test]
    fn test_ser_join() -> anyhow::Result<()> {
        let join_ix = RaceInstruction::JoinGame{
            params: JoinParams {
                amount: 1000u64,
                access_version: 0u64,
                position: 2u16,
            }
        };

        let join_ix_ser = join_ix.try_to_vec().unwrap();
        println!("join ix serialized {:?}", join_ix);
        let join_bytes = [10, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0];
        assert_eq!(join_ix_ser, join_bytes);


        Ok(())
    }
}
