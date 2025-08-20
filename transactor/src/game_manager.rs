use race_api::event::{Event, Message};
use race_core::checkpoint::CheckpointOffChain;
use race_core::context::SubGameInit;
use race_core::error::{Error, Result};
use race_core::types::{BroadcastFrame, ClientMode, ServerAccount};
use race_encryptor::Encryptor;
use race_env::TransactorConfig;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::blacklist::Blacklist;
use crate::component::{CheckpointBroadcastFrame, BridgeToParent, CloseReason, WrappedStorage, WrappedTransport};
use crate::frame::{EventFrame, SignalFrame};
use crate::handle::Handle;
use crate::utils::current_timestamp;

pub struct GameManager {
    games: Arc<RwLock<HashMap<String, Handle>>>,
}

impl Default for GameManager {
    fn default() -> Self {
        Self {
            games: Arc::new(RwLock::new(HashMap::default())),
        }
    }
}

impl GameManager {
    /// Load a child game
    pub async fn launch_sub_game(
        &self,
        sub_game_init: SubGameInit,
        bridge_to_parent: BridgeToParent,
        server_account: &ServerAccount,
        transport: Arc<WrappedTransport>,
        encryptor: Arc<Encryptor>,
        storage: Arc<WrappedStorage>,
        signal_tx: mpsc::Sender<SignalFrame>,
        config: &TransactorConfig,
    ) -> Option<JoinHandle<CloseReason>> {
        let game_addr = sub_game_init.spec.game_addr.clone();
        let game_id = sub_game_init.spec.game_id;
        match &sub_game_init.source {
            race_core::context::SubGameInitSource::FromCheckpoint(ref versioned_data) => {
                info!(
                    "LaunchSubGame with Checkpoint: version = {}, bundle = {}",
                    versioned_data.versions, sub_game_init.spec.bundle_addr
                );
            }
            race_core::context::SubGameInitSource::FromInitAccount(_, ref versions) => {
                info!(
                    "LaunchSubGame with InitAccount: version = {}, bundle = {}",
                    versions, sub_game_init.spec.bundle_addr
                );
            }
        }
        match Handle::try_new_sub_game(
            sub_game_init,
            bridge_to_parent,
            transport,
            encryptor,
            storage,
            server_account,
            config,
        )
        .await
        {
            Ok(mut handle) => {
                let mut games = self.games.write().await;
                let addr = format!("{}:{}", game_addr, game_id);
                info!("Launch child game {}", addr);
                let join_handle = handle.wait(signal_tx);
                games.insert(addr.clone(), handle);
                Some(join_handle)
            }
            Err(e) => {
                warn!(
                    "Error loading child game with id {}: {}",
                    game_id,
                    e.to_string()
                );
                None
            }
        }
    }

    /// Load game by its address.  This operation is idempotent.
    pub async fn launch_game(
        &self,
        game_addr: String,
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        blacklist: Arc<Mutex<Blacklist>>,
        signal_tx: mpsc::Sender<SignalFrame>,
        mode: ClientMode,
        config: &TransactorConfig,
    ) -> Option<JoinHandle<CloseReason>> {
        let handle = if mode == ClientMode::Transactor {
            Handle::try_new_transactor(
                transport,
                storage,
                encryptor,
                server_account,
                &game_addr,
                signal_tx.clone(),
                &config,
            )
                .await
        } else {
            Handle::try_new_validator(
                transport,
                storage,
                encryptor,
                server_account,
                &game_addr,
                signal_tx.clone(),
                config,
            )
                .await
        };

        let mut handle = match handle {
            Ok(handle) => {
                info!("Game handle created: {}", handle.addr());
                handle
            }
            Err(err) => {
                warn!("Error loading game: {}", err.to_string());
                warn!("Failed to load game: {}", game_addr);
                blacklist.lock().await.add_addr(&game_addr);
                return None
            }
        };

        let mut games = self.games.write().await;
        if let Entry::Vacant(e) = games.entry(game_addr.clone()) {
            let join_handle = handle.wait(signal_tx);
            e.insert(handle);
            Some(join_handle)
        } else {
            error!("Game already loaded: {}", game_addr);
            None
        }
    }

    pub async fn get_serving_addrs(&self) -> Vec<String> {
        let games = self.games.read().await;
        games.keys().cloned().collect()
    }

    pub async fn is_game_loaded(&self, game_addr: &str) -> bool {
        let games = self.games.read().await;
        games.contains_key(game_addr)
    }

    pub async fn send_event(&self, game_addr: &str, event: Event) -> Result<()> {
        let games = self.games.read().await;
        if let Some(handle) = games.get(game_addr) {
            let timestamp = current_timestamp();
            let event_frame = EventFrame::SendEvent { event, timestamp };
            handle.event_bus().send(event_frame).await;
            Ok(())
        } else {
            warn!("Game {} not loaded, discard event: {:?}", game_addr, event);
            Err(Error::GameNotLoaded)
        }
    }

    pub async fn send_message(&self, game_addr: &str, message: Message) -> Result<()> {
        let games = self.games.read().await;
        if let Some(handle) = games.get(game_addr) {
            let event_frame = EventFrame::SendMessage { message };
            handle.event_bus().send(event_frame).await;
            Ok(())
        } else {
            warn!(
                "Game {} not loaded, discard message: {:?}",
                game_addr, message
            );
            Err(Error::GameNotLoaded)
        }
    }

    pub async fn eject_player(&self, game_addr: &str, player_addr: &str) -> Result<()> {
        let games = self.games.read().await;
        if let Some(handle) = games.get(game_addr) {
            info!(
                "Receive leaving request from {:?} for game {:?}",
                player_addr, game_addr
            );
            let event_frame = EventFrame::PlayerLeaving {
                player_addr: player_addr.to_owned(),
            };
            handle.event_bus().send(event_frame).await;
            Ok(())
        } else {
            warn!("Game not loaded, discard leaving request");
            Err(Error::GameNotLoaded)
        }
    }

    pub async fn get_checkpoint(
        &self,
        game_addr: &str,
        settle_version: u64,
    ) -> Result<Option<CheckpointOffChain>> {
        let games = self.games.read().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        let broadcaster = handle.broadcaster()?;
        let checkpoint = broadcaster.get_checkpoint(settle_version).await;
        Ok(checkpoint)
    }

    pub async fn get_latest_checkpoint(
        &self,
        game_addr: &str,
    ) -> Result<Option<CheckpointOffChain>> {
        let games = self.games.read().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        let broadcaster = handle.broadcaster()?;
        let checkpoint = broadcaster.get_latest_checkpoint().await;
        Ok(checkpoint)
    }

    /// Get the broadcast channel of game, and its event histories
    pub async fn get_broadcast_and_backlogs(
        &self,
        game_addr: &str,
        settle_version: u64,
    ) -> Result<(broadcast::Receiver<BroadcastFrame>, BroadcastFrame)> {
        let games = self.games.read().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        let broadcaster = handle.broadcaster()?;
        let receiver = broadcaster.get_broadcast_rx();
        let backlogs = broadcaster.get_backlogs(settle_version).await;
        Ok((receiver, backlogs))
    }

    /// Get the checkopint channel of game and its latest checkpoint
    pub async fn get_broadcast_and_checkpoint(
        &self,
        game_addr: &str,
    ) -> Result<(broadcast::Receiver<CheckpointBroadcastFrame>, CheckpointBroadcastFrame)> {
        let games = self.games.read().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        let broadcaster = handle.broadcaster()?;
        let receiver = broadcaster.get_checkpoint_rx();
        let Some(frame) = broadcaster.get_latest_checkpoint_broadcast_frame().await else {
            return Err(Error::MissingCheckpoint);
        };

        Ok((receiver, frame))
    }

    pub async fn remove_game(&self, addr: &str) {
        let mut games = self.games.write().await;
        games.remove(&addr.to_string());
    }

    /// Shutdown all games, and drop their handles.
    pub async fn shutdown(&self) {
        let mut games = self.games.write().await;
        for (addr, game) in games.iter() {
            if !game.is_subgame() {
                info!("Shutdown game {}", addr);
                game.event_bus().send(EventFrame::Shutdown).await;
            }
        }

        games.clear();
    }
}
