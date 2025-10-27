use race_env::Config;
use std::sync::Arc;
use crate::session_manager::SessionManager;
use crate::error::ReplayerError;
use tracing::info;

pub struct ReplayerContext {
    pub config: Config,
    pub session_manager: SessionManager,
}

impl ReplayerContext {
    pub fn new(config: Config) -> Self {
        let session_manager = SessionManager::new();
        Self { config, session_manager }
    }
}

impl ReplayerContext {

    // Start a replay session
    pub async fn launch_replay_session(player_addr: String, game_addr: String) -> Result<(), ReplayerError> {
        info!("Launch replay session for player: {} game: {}", player_addr, game_addr);

        Ok(())
    }
}
