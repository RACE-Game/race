use crate::blacklist::Blacklist;
use crate::component::{CloseReason, WrappedStorage, WrappedTransport};
use crate::frame::SignalFrame;
use crate::game_manager::GameManager;
use race_api::event::{Event, Message};
use race_core::error::{Error, Result};
use race_core::encryptor::{EncryptorT, NodePublicKeyRaw};
use race_core::transport::TransportT;
use race_core::types::{BroadcastFrame, ServerAccount, Signature};
use race_encryptor::Encryptor;
use race_env::{Config, TransactorConfig};
use race_transport::ChainType;
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch, Mutex};
use tracing::{error, info};

/// Transactor runtime context
pub struct ApplicationContext {
    pub config: TransactorConfig,
    pub chain: ChainType,
    pub account: ServerAccount,
    pub transport: Arc<WrappedTransport>,
    pub storage: Arc<WrappedStorage>,
    pub encryptor: Arc<Encryptor>,
    pub game_manager: Arc<GameManager>,
    pub signal_tx: mpsc::Sender<SignalFrame>,
    pub blacklist: Arc<Mutex<Blacklist>>,
    pub shutdown_rx: watch::Receiver<bool>,
}

impl ApplicationContext {
    pub async fn try_new_and_start_signal_loop(config: Config) -> Result<(Self, JoinHandle<()>)> {
        info!("Initialize application context");

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

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

        let game_manager = Arc::new(GameManager::default());

        let (signal_tx, signal_rx) = mpsc::channel(3);

        let blacklist = Arc::new(Mutex::new(Blacklist::new(
            transactor_config.disable_blacklist.ne(&(Some(true))),
        )));

        let ctx = Self {
            config: transactor_config,
            chain,
            account,
            transport,
            storage,
            encryptor,
            game_manager,
            signal_tx,
            blacklist,
            shutdown_rx,
        };

        let join_handle = ctx.start_signal_loop(signal_rx, shutdown_tx);

        Ok((ctx, join_handle))
    }

    pub fn start_signal_loop(&self, mut signal_rx: mpsc::Receiver<SignalFrame>, shutdown_tx: watch::Sender<bool>) -> JoinHandle<()> {
        info!("Starting signal loop");

        let game_manager_0 = self.game_manager.clone();
        let transport_0 = self.transport.clone();
        let storage_0 = self.storage.clone();
        let encryptor_0 = self.encryptor.clone();
        let account_0 = self.account.clone();
        let blacklist_0 = self.blacklist.clone();
        let signal_tx_0 = self.signal_tx.clone();
        let config_0 = self.config.clone();

        tokio::spawn(async move {
            let mut join_handles: Vec<JoinHandle<CloseReason>> = vec![];

            while let Some(signal) = signal_rx.recv().await {

                let game_manager_1 = game_manager_0.clone();
                let transport_1 = transport_0.clone();
                let storage_1 = storage_0.clone();
                let encryptor_1 = encryptor_0.clone();
                let account_1 = account_0.clone();
                let blacklist_1 = blacklist_0.clone();
                let signal_tx_1 = signal_tx_0.clone();

                match signal {
                    SignalFrame::StartGame { game_addr, mode } => {
                        if let Some(join_handle) = game_manager_1
                            .launch_game(
                                game_addr,
                                transport_1.clone(),
                                storage_1.clone(),
                                encryptor_1.clone(),
                                &account_1,
                                blacklist_1.clone(),
                                signal_tx_1.clone(),
                                mode,
                                &config_0,
                            )
                            .await {
                                join_handles.push(join_handle);
                            }
                    }
                    SignalFrame::LaunchSubGame { sub_game_init, bridge_to_parent } => {
                        if let Some(join_handle) = game_manager_1
                            .launch_sub_game(
                                sub_game_init,
                                bridge_to_parent,
                                &account_1,
                                transport_1.clone(),
                                encryptor_1.clone(),
                                storage_1.clone(),
                                signal_tx_1.clone(),
                                &config_0,
                            )
                            .await {
                                join_handles.push(join_handle);
                            }
                    }

                    SignalFrame::Shutdown => {
                        game_manager_1.shutdown().await;
                        shutdown_tx.send(true).expect("Set shutdown flag");
                        break;
                    }

                    SignalFrame::RemoveGame { game_addr } => {
                        info!("Unload game {}", game_addr);
                        game_manager_1.remove_game(&game_addr).await;
                    }
                }
            }

            info!("Waiting game handles to finish...");

            for join_handle in join_handles {
                if let Err(e) = join_handle.await {
                    error!("Error in waiting game handles: {}", e);
                }
            }
            info!("All game handles stopped");
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

    pub fn get_shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }

    pub fn blacklist(&self) -> Arc<Mutex<Blacklist>> {
        self.blacklist.clone()
    }
 }
