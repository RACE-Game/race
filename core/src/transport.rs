#[allow(unused_imports)]
use crate::error::Error;
use crate::{
    error::Result,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
        CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, PublishParams, RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams,
        ServerAccount, SettleParams, UnregisterGameParams, VoteParams,
    },
};
use async_trait::async_trait;

#[async_trait]
pub trait TransportT: Send + Sync {
    /// Create an on-chain game account which represents a game room
    /// and holds basic game properties.  Check [`GameAccount`] for
    /// description of the layout. The implementation should contain
    /// the check for transaction signature, make sure that Ok is
    /// returned only when the transaction succeeds and finalized.
    ///
    /// # Arguments
    /// * `max_players` - The maximum number of players in this game. Please note,
    ///   not all games require a full room to start.
    /// * `bundle_addr` - The address of game bundle NFT.
    /// * `data` - The borsh serialization of game specific data. The layout should be
    ///   game independent. It is used to describe the basic game properties,
    ///   and is considered to be immutable.
    ///
    /// # Returns
    /// * [`Error::InvalidMaxPlayers`] when invalid `max_players` is provided.
    /// * [`Error::GameBundleNotFound`] when invalid `bundle_addr` is provided.
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String>;

    /// Close the game account.  A game can be closed when it's empty
    /// (no players).  To close the game, the signer must be the owner
    /// of the account.
    ///
    /// # Arguments
    /// * `addr` - The address of game to be closed.
    ///
    /// # Returns
    /// * [`Error::GameAccountNotFound`] when invalid `addr` is provided.
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()>;

    /// Create an on-chain account for server which can serve game
    /// accounts.  Check [`ServerAccount`] for the description of the
    /// layout. The owner is limited to have only one server.
    ///
    /// # Arguments
    /// * `owner_addr` - The account of the server owner, should be the same to the signer.
    /// * `endpoint` - The accessible endpoint to public.  The format shouldn't contain the protocol.
    ///   e.g. 127.0.0.1:8000, example.org
    ///
    /// # Returns
    /// * [`Error::ServerAccountExists`] when the account has been created already.
    /// * [`Error::MalformedEndpoint`] when the `endpoint` is invalid.
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn register_server(&self, params: RegisterServerParams) -> Result<String>;

    /// Join the game.
    ///
    /// # Arguments
    /// * `player_addr` - The address of player, should the same with signer.
    /// * `game_addr` - The game to join.
    /// * `amount` - The amount of token to bring to the game.
    /// * `access_version` - The current access version.
    /// * `position` - The position to be at in the game, should be an index,
    ///   must be less than the `max_players` of the game account.
    ///
    /// # Returns
    /// * [`Error::GameAccountNotFound`] when invalid `game_addr` is provided.
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn join(&self, params: JoinParams) -> Result<()>;

    /// Deposit tokens into game.
    ///
    /// # Arguments
    /// * `player_addr` - The address of player, should be the same with signer.
    /// * `game_addr` - The game to deposit.
    /// * `amount` - The amount of token to deposit.
    /// * `access_version` - The current access version.
    async fn deposit(&self, params: DepositParams) -> Result<()>;

    /// Serve a game.  To serve a game, server will write its address into game account.
    ///
    /// # Arguments
    /// * `game_addr` - The address of game to serve.
    /// * `server_addr` - The address of server, should be the same with signer.
    ///
    /// # Returns
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn serve(&self, params: ServeParams) -> Result<()>;

    /// Send a vote to game account.  For example, vote for a server disconnecting.
    ///
    /// # Arguments:
    /// * `vote_type` - The type of vote, currently only `ServerDropOff` and `ServerIsOnline` are supported.
    /// * `sender_addr` - The sender of the vote, must be the same with signer.
    /// * `receiver_addr` - The receiver of the vote.  Generally, it should be the server address.
    async fn vote(&self, params: VoteParams) -> Result<()>;

    /// Create a player profile on chain.  A profile is required to join any games.
    /// The player profile address is derived from the player wallet address.
    ///
    /// # Arguments
    /// * `addr` - The address of the wallet, should be the same with signer.
    /// * `nick` - The display name in the game, can't be empty.
    /// * `pfp` - The address of the NFT token to be used.  `None` means using default pfp.
    ///
    /// # Returns
    /// * [`Error::PlayerProfileAccountNotFound`] when invalid `addr` is provided.
    /// * [`Error::RpcError`] when the RPC invocation failed.
    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<String>;

    async fn publish_game(&self, params: PublishParams) -> Result<String>;

    async fn settle_game(&self, params: SettleParams) -> Result<()>;

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String>;

    async fn register_game(&self, params: RegisterGameParams) -> Result<()>;

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()>;

    /// Get game account by its address.
    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    /// Get game bundle account by its address.
    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    /// Get player profile account by its address.
    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile>;

    /// Get server account by its address.
    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount>;

    /// Get registration account by its address.
    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount>;
}
