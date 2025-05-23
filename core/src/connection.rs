use crate::error::Result;
use crate::types::{AttachGameParams, ExitGameParams, SubmitEventParams};
use async_trait::async_trait;

#[async_trait]
pub trait ConnectionT: Sync + Send{
    /// Attach to game. While processing this request, transactor will load
    /// the game into memory if it hasn't already been loaded.
    async fn attach_game(&self, game_addr: &str, params: AttachGameParams) -> Result<()>;

    /// Submit event to transactor.
    async fn submit_event(&self, game_addr: &str, params: SubmitEventParams) -> Result<()>;

    /// Exit game.  The request will fail if it's not allowed to quit at the moment.
    async fn exit_game(&self, game_addr: &str, params: ExitGameParams) -> Result<()>;
}
