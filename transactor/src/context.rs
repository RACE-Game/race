use crate::blacklist::Blacklist;
use crate::component::{WrappedStorage, WrappedTransport};
use crate::frame::SignalFrame;
use crate::game_manager::GameManager;
use race_api::error::{Error, Result};
use race_api::event::{Event, Message};
use race_core::encryptor::{EncryptorT, NodePublicKeyRaw};
use race_core::transport::TransportT;
use race_core::types::{BroadcastFrame, ServerAccount, Signature};
use race_encryptor::Encryptor;
use race_env::{Config, TransactorConfig};
use race_transport::ChainType;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;

/// Transactor runtime context
pub struct ApplicationContext {
    pub config: TransactorConfig,
    pub chain: ChainType,
    pub account: ServerAccount,
    pub transport: Arc<WrappedTransport>,
    pub encryptor: Arc<Encryptor>,
    pub game_manager: Arc<GameManager>,
    pub signal_tx: mpsc::Sender<SignalFrame>,
    pub blacklist: Arc<Mutex<Blacklist>>,
}

impl ApplicationContext {
    pub async fn try_new(config: Config) -> Result<Self> {
        info!("Initialize application context");

        let transport = Arc::new(WrappedTransport::try_new(&config).await?);

        let storage = Arc::new(WrappedStorage::try_new(&config).await?);

        let encryptor = Arc::new(Encryptor::default());

        let transactor_config = config.transactor.ok_or(Error::TransactorConfigMissing)?;

        let chain: ChainType = transactor_config.chain.as_str().try_into()?;

        info!("Transactor wallet address: {}", transactor_config.address);

        let account = transport
            .get_server_account(&transactor_config.address)
            .await?
            .ok_or(Error::ServerAccountMissing)?;

        let debug_mode = transactor_config.debug_mode.unwrap_or(false);

        let game_manager = Arc::new(GameManager::default());

        let (signal_tx, mut signal_rx) = mpsc::channel(3);

        let blacklist = Arc::new(Mutex::new(Blacklist::new(
            transactor_config.disable_blacklist.ne(&(Some(true))),
        )));

        let game_manager_0 = game_manager.clone();
        let transport_0 = transport.clone();
        let storage_0 = storage.clone();
        let encryptor_0 = encryptor.clone();
        let account_0 = account.clone();
        let blacklist_0 = blacklist.clone();
        let signal_tx_0 = signal_tx.clone();

        tokio::spawn(async move {
            while let Some(signal) = signal_rx.recv().await {
                let game_manager_1 = game_manager_0.clone();
                let transport_1 = transport_0.clone();
                let storage_1 = storage_0.clone();
                let encryptor_1 = encryptor_0.clone();
                let account_1 = account_0.clone();
                let blacklist_1 = blacklist_0.clone();
                let signal_tx_1 = signal_tx_0.clone();
                tokio::spawn(async move {
                    match signal {
                        SignalFrame::StartGame { game_addr } => {
                            game_manager_1
                                .load_game(
                                    game_addr,
                                    transport_1.clone(),
                                    storage_1.clone(),
                                    encryptor_1.clone(),
                                    &account_1,
                                    blacklist_1.clone(),
                                    signal_tx_1.clone(),
                                    debug_mode,
                                )
                                .await;
                        }
                        SignalFrame::LaunchSubGame { spec, checkpoint } => {
                            let bridge_parent = game_manager_1
                                .get_event_parent(&spec.game_addr)
                                .await
                                .expect(
                                    format!("Bridge parent not found: {}", spec.game_addr).as_str(),
                                );

                            game_manager_1
                                .launch_sub_game(
                                    spec,
                                    checkpoint,
                                    bridge_parent,
                                    &account_1,
                                    transport_1.clone(),
                                    encryptor_1.clone(),
                                    debug_mode,
                                )
                                .await;
                        }
                    }
                });
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
            blacklist,
        })
    }

    pub async fn register_key(&self, player_addr: String, key: NodePublicKeyRaw) -> Result<()> {
        self.encryptor.add_public_key(player_addr, &key)?;
        Ok(())
    }

    pub fn export_public_key(&self) -> NodePublicKeyRaw {
        self.encryptor
            .export_public_key(None)
            .expect("Export public key failed")
    }

    pub fn verify(&self, arg: &[u8], signature: &Signature) -> Result<()> {
        self.encryptor.verify(arg, signature)?;
        Ok(())
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

    pub async fn send_message(&self, game_addr: &str, message: Message) -> Result<()> {
        self.game_manager.send_message(game_addr, message).await
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

    pub fn get_signal_sender(&self) -> mpsc::Sender<SignalFrame> {
        self.signal_tx.clone()
    }

    pub fn blacklist(&self) -> Arc<Mutex<Blacklist>> {
        self.blacklist.clone()
    }
}
