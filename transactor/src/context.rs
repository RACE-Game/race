use crate::component::WrappedTransport;
use crate::frame::{EventFrame, SignalFrame};
use crate::handle::Handle;
use race_core::encryptor::EncryptorT;
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::{BroadcastFrame, ServerAccount, Signature};
use race_encryptor::Encryptor;
use race_env::{Config, TransactorConfig};
use race_transport::ChainType;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;
use tracing::log::warn;

pub struct GameManager {
    games: Mutex<HashMap<String, Handle>>,
}

impl Default for GameManager {
    fn default() -> Self {
        Self {
            games: Mutex::new(HashMap::default()),
        }
    }
}

impl GameManager {
    /// Load game by its address.  This operation is idempotent.
    pub async fn load_game(
        &self,
        game_addr: String,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        server_account: &ServerAccount,
    ) {
        let mut games = self.games.lock().await;
        if let Entry::Vacant(e) = games.entry(game_addr) {
            if let Ok(handle) = Handle::try_new(transport, encryptor, server_account, e.key()).await
            {
                info!("Game handle created: {}", e.key());
                e.insert(handle);
            } else {
                warn!("Failed to load game: {}", e.key());
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
        let receiver = handle.broadcaster()?.get_broadcast_rx();
        let histories = handle
            .broadcaster()?
            .retrieve_histories(settle_version)
            .await;
        Ok((receiver, histories))
    }

    pub async fn get_snapshot(&self, game_addr: &str) -> Result<String> {
        let games = self.games.lock().await;
        let handle = games.get(game_addr).ok_or(Error::GameNotLoaded)?;
        Ok(handle.broadcaster()?.get_snapshot().await)
    }
}

/// Transactor runtime context
pub struct ApplicationContext {
    pub config: TransactorConfig,
    pub chain: ChainType,
    pub account: ServerAccount,
    pub transport: Arc<dyn TransportT>,
    pub encryptor: Arc<dyn EncryptorT>,
    pub game_manager: Arc<GameManager>,
    pub signal_tx: mpsc::Sender<SignalFrame>,
}

impl ApplicationContext {
    pub async fn try_new(config: Config) -> Result<Self> {
        info!("Initialize application context");

        let transport = Arc::new(WrappedTransport::try_new(&config).await?);

        let encryptor = Arc::new(Encryptor::default());

        let transactor_config = config.transactor.ok_or(Error::TransactorConfigMissing)?;

        let chain: ChainType = transactor_config.chain.as_str().try_into()?;

        info!("Transactor wallet address: {}", transactor_config.address);

        let account = transport
            .get_server_account(&transactor_config.address)
            .await
            .ok_or(Error::ServerAccountMissing)?;

        let game_manager = Arc::new(GameManager::default());
        let game_manager_1 = game_manager.clone();

        let (signal_tx, mut signal_rx) = mpsc::channel(3);

        let transport_1 = transport.clone();
        let encryptor_1 = encryptor.clone();
        let account_1 = account.clone();

        tokio::spawn(async move {
            while let Some(signal) = signal_rx.recv().await {
                match signal {
                    SignalFrame::StartGame { game_addr } => {
                        game_manager_1
                            .load_game(
                                game_addr,
                                transport_1.clone(),
                                encryptor_1.clone(),
                                &account_1,
                            )
                            .await;
                    }
                }
            }
        });

        Ok(Self {
            config: transactor_config,
            chain,
            account,
            transport,
            encryptor,
            game_manager,
            signal_tx,
        })
    }

    pub async fn register_key(&self, player_addr: String, key: String) -> Result<()> {
        info!("Client {:?} register public key, {}", player_addr, key);
        self.encryptor.add_public_key(player_addr, &key)?;
        Ok(())
    }

    pub async fn verify<S: ToString>(
        &self,
        game_addr: &str,
        arg: &S,
        signature: &Signature,
    ) -> Result<()> {
        let message = format!("{}{}", game_addr, arg.to_string());
        // info!("Verify, message: \"{}\", signature: {}", message, signature);
        Ok(self.encryptor.verify(&message.as_bytes(), signature)?)
    }

    /// Return if the game is loaded.
    #[allow(unused)]
    pub async fn is_game_loaded(&self, game_addr: &str) -> bool {
        self.game_manager.is_game_loaded(game_addr).await
    }

    pub async fn eject_player(&self, game_addr: &str, player_addr: &str) -> Result<()> {
        self.game_manager.eject_player(game_addr, player_addr).await
    }

    pub async fn send_event(&self, game_addr: &str, event: Event) -> Result<()> {
        self.game_manager.send_event(game_addr, event).await
    }

    pub async fn get_broadcast(
        &self,
        game_addr: &str,
        settle_version: u64,
    ) -> Result<(broadcast::Receiver<BroadcastFrame>, Vec<BroadcastFrame>)> {
        self.game_manager
            .get_broadcast(game_addr, settle_version)
            .await
    }

    pub async fn get_snapshot(&self, game_addr: &str) -> Result<String> {
        self.game_manager.get_snapshot(game_addr).await
    }

    pub fn get_signal_sender(&self) -> mpsc::Sender<SignalFrame> {
        self.signal_tx.clone()
    }
}
