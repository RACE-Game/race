//! Copied from race-solana and modified

use super::{
    IxAssignRecipientParams, IxCreateGameAccountParams, IxCreatePlayerProfileParams,
    IxCreateRecipientParams, IxCreateRegistrationParams, IxJoinParams, IxPublishParams,
    IxRegisterServerParams, IxServeParams, IxSettleParams, IxVoteParams, IxRejectDepositsParams
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum RaceInstruction {
    /// # Create a new game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of transactor
    /// 1. `[writable]` The game account, hold all necessary info about the game
    /// 2. `[writable]` The temp stake account
    /// 3. `[]` The mint account
    /// 4. `[]` The token program
    /// 5. `[]` The bundled data account
    /// 6. `[]` The recipient account
    CreateGameAccount { params: IxCreateGameAccountParams },

    /// # Close a new game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[]` The account of game account
    /// 2. `[writable]` The game reg account
    /// 3. `[writable]` The stake account of game
    /// 4. `[]` PDA account.
    /// 5. `[]` Token program.
    CloseGameAccount,

    /// # Create an on-chain "lobby" for game registration
    ///
    /// Accounts expected:
    /// 0. `[signer]` The account of game owner
    /// 1. `[writable]` The registry account
    CreateRegistry { params: IxCreateRegistrationParams },

    /// # Create a player profile
    ///
    /// Accounts expected:
    /// 0. `[signer]` The owner of the player profile
    /// 1. `[]` The player profile account to be created
    /// 2. `[]` The pfp account
    CreatePlayerProfile { params: IxCreatePlayerProfileParams },

    /// # Register (Create) a server profile
    ///
    /// Accounts expected:
    /// 0. `[signer]` The owner of the player profile
    /// 1. `[]` The server profile account to be created
    RegisterServer { params: IxRegisterServerParams },

    /// # Settle game result
    ///
    /// Accounts expected:
    /// 0. `[signer]` The game transactor account
    /// 1. `[writable]` The game account
    /// 2. `[writable]` The stake account, must match the one in game account
    /// 3. `[]` PDA account
    /// 4. `[]` The recipient account
    /// 5. `[]` The token program
    /// 6. `[]` The system program
    /// Following:
    /// `[]` Every leaving players account, must be in the same order with Eject settles
    /// `[]` Every recipient slot accounts to receive transfer
    Settle { params: IxSettleParams },

    /// # Vote
    ///
    /// Accounts expected:
    /// 0. `[signer]` The voter account, could be the wallet address of a server or a player.
    /// 1. `[writable]` The game account.
    /// 2. `[]` The votee account.
    Vote { params: IxVoteParams },

    /// # Serve a game
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer acount (the server itself)
    /// 1. `[writable]` The game account to be served
    /// 2. `[]` The server account
    ServeGame { params: IxServeParams },

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
    JoinGame { params: IxJoinParams },

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
    PublishGame { params: IxPublishParams },

    /// # Create recipient
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer account
    /// 1. `[]` The cap account
    /// 2. `[]` The recipient account
    /// 3. `[]` The token program
    /// 3+n. `[]` The Nth staking account for slots
    CreateRecipient { params: IxCreateRecipientParams },

    /// # Assign recipient
    ///
    /// Accounts expected:
    /// 0. `[signer]` The payer account, should be the cap account of recipient
    /// 1. `[writable]` The recipient account
    /// 2. `[]` The account to assigned as the owner to a slot
    AssignRecipient { params: IxAssignRecipientParams },

    /// # Recipient claim
    ///
    /// Accounts expected:
    /// 0. `[signer]` The fee payer
    /// 1. `[writable]` The recipient account
    /// 2. `[]` The PDA account as the owner of stake accounts
    /// 3. `[]` The token program
    /// 4. `[]` The system program
    /// Rest. `[]` The stake account followed by the corresponding ATA to receive tokens
    RecipientClaim,


    /// #[17] Reject a deposit
    ///
    /// Accounts expected:
    /// 0. `[signer]` The transactor account
    /// 1. `[writable]` The game account
    /// 2. `[]` The stake account
    /// 3. `[]` The PDA from game account
    /// 4. `[]` The SPL token program
    /// 5. `[]` The system program
    /// Rest. `[writable]` The receiver for each rejected deposit
    RejectDeposits { params: IxRejectDepositsParams },
}
