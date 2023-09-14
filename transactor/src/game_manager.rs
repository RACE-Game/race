use race_core::error::{Error, Result};
use race_core::event::{Event, Message};
use race_core::types::{BroadcastFrame, ServerAccount};
use race_encryptor::Encryptor;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};

use crate::blacklist::Blacklist;
use crate::component::{CloseReason, WrappedTransport};
use crate::frame::EventFrame;
use crate::handle::Handle;

pub struct GameManager {
    games: Arc<Mutex<HashMap<String, Handle>>>,
}

impl Default for GameManager {
    fn default() -> Self {
        Self {
            games: Arc::new(Mutex::new(HashMap::default())),
        }
    }
}

impl GameManager {
    /// Load game by its address.  This operation is idempotent.
    pub async fn load_game(
        &self,
        game_addr: String,
        transport: Arc<WrappedTransport>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        blacklist: Arc<Mutex<Blacklist>>,
    ) {
        let mut games = self.games.lock().await;
        if let Entry::Vacant(e) = games.entry(game_addr.clone()) {
            match Handle::try_new(transport, encryptor, server_account, e.key()).await {
                Ok(mut handle) => {
                    info!("Game handle created: {}", e.key());
                    let join_handle = handle.wait();
                    e.insert(handle);
                    let games = self.games.clone();
                    // Wait and unload
                    tokio::spawn(async move {
                        match join_handle.await {
                            Ok(CloseReason::Complete) => {
                                let mut games = games.lock().await;
                                games.remove(&game_addr);
                                info!("Clean game handle: {}", game_addr);
                            }
                            Ok(CloseReason::Fault(_)) => {
                                let mut games = games.lock().await;
                                games.remove(&game_addr);
                                blacklist.lock().await.add_addr(&game_addr);
                                info!("Game stopped with error, clean game handle: {}", game_addr);
                            }
                            Err(e) => {
                                error!("Unexpected error when waiting game to stop: {}", e);
                            }
                        }
                    });
                }
                Err(err) => {
                    warn!("Error loading game: {}", err.to_string());
                    warn!("Failed to load game: {}", e.key());
                }
            }
        }
    }

    pub async fn is_game_loaded(&self, game_addr: &str) -> bool {
        let games = self.games.lock().await;
        games.get(game_addr).is_some()
    }

    pub async fn send_event(&self, game_addr: &str, event: Event) -> Result<()> {
        let games = self.games.lock().await;
        if let Some(handle) = games.get(game_addr) {
            let event_frame = EventFrame::SendEvent { event };
            handle.event_bus().send(event_frame).await;
            Ok(())
        } else {
            warn!("Game {} not loaded, discard event: {:?}", game_addr, event);
            Err(Error::GameNotLoaded)
        }
    }

    pub async fn send_message(&self, game_addr: &str, message: Message) -> Result<()> {
        let games = self.games.lock().await;
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
        let games = self.games.lock().await;
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

    /// Get the broadcast channel of game, and its event histories
    pub async fn get_broadcast(
        &self,
        game_addr: &str,
        settle_version: u64,
    ) -> Result<(broadcast::Receiver<BroadcastFrame>, Vec<BroadcastFrame>)> {
        let games = self.games.lock().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        let broadcaster = handle.broadcaster()?;
        let receiver = broadcaster.get_broadcast_rx();
        let histories = broadcaster.retrieve_histories(settle_version).await;
        Ok((receiver, histories))
    }
}
