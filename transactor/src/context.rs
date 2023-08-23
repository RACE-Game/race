use crate::component::WrappedTransport;
use crate::frame::SignalFrame;
use crate::game_manager::GameManager;
use race_core::encryptor::{EncryptorT, NodePublicKeyRaw};
use race_core::error::{Error, Result};
use race_core::event::{Event, Message};
use race_core::transport::TransportT;
use race_core::types::{BroadcastFrame, ServerAccount, Signature};
use race_encryptor::Encryptor;
use race_env::{Config, TransactorConfig};
use race_transport::ChainType;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
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
            .await?
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

    pub async fn register_key(&self, player_addr: String, key: NodePublicKeyRaw) -> Result<()> {
        // info!("Client {:?} register public key, {:?}", player_addr, key);
        self.encryptor.add_public_key(player_addr, &key)?;
        Ok(())
    }

    pub fn export_public_key(&self) -> NodePublicKeyRaw {
        self.encryptor.export_public_key(None).expect("Export public key failed").clone()
    }

    pub fn verify(
        &self,
        arg: &[u8],
        signature: &Signature,
    ) -> Result<()> {
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
}
