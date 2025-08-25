mod subgame;
mod transactor;
mod validator;

use std::sync::Arc;

use race_transactor_frames::{BridgeToParent, SignalFrame};
use race_transactor_components::{
    Broadcaster, CloseReason, EventBus, WrappedStorage, WrappedTransport,
};
use race_core::context::SubGameInit;
use race_core::error::{Error, Result};
use race_core::types::ServerAccount;
use race_encryptor::Encryptor;
use race_env::TransactorConfig;
use subgame::SubGameHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::error;
use transactor::TransactorHandle;
use validator::ValidatorHandle;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
    SubGame(SubGameHandle),
}

impl Handle {
    pub async fn try_new_transactor(
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        game_addr: &str,
        signal_tx: mpsc::Sender<SignalFrame>,
        config: &TransactorConfig,
    ) -> Result<Self> {
        Ok(Self::Transactor(
            TransactorHandle::try_new(
                game_addr,
                server_account,
                encryptor,
                transport,
                storage,
                signal_tx,
                config,
            )
            .await?,
        ))
    }

    pub async fn try_new_validator(
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        game_addr: &str,
        signal_tx: mpsc::Sender<SignalFrame>,
        config: &TransactorConfig,
    ) -> Result<Self> {
        Ok(Self::Validator(
            ValidatorHandle::try_new(
                game_addr,
                server_account,
                encryptor,
                transport,
                storage,
                signal_tx,
                config,
            )
            .await?,
        ))
    }

    pub async fn try_new_sub_game(
        sub_game_init: SubGameInit,
        bridge_to_parent: BridgeToParent,
        transport: Arc<WrappedTransport>,
        encryptor: Arc<Encryptor>,
        storage: Arc<WrappedStorage>,
        server_account: &ServerAccount,
        config: &TransactorConfig,
    ) -> Result<Self> {
        Ok(Self::SubGame(
            SubGameHandle::try_new(
                sub_game_init,
                bridge_to_parent,
                transport,
                encryptor,
                storage,
                server_account,
                config,
            )
            .await?,
        ))
    }

    pub fn broadcaster(&self) -> Result<&Broadcaster> {
        match self {
            Handle::Transactor(h) => Ok(&h.broadcaster),
            Handle::Validator(_) => Err(Error::NotSupportedInValidatorMode),
            Handle::SubGame(h) => Ok(&h.broadcaster),
        }
    }

    pub fn event_bus(&self) -> &EventBus {
        match self {
            Handle::Transactor(h) => &h.event_bus,
            Handle::Validator(h) => &h.event_bus,
            Handle::SubGame(h) => &h.event_bus,
        }
    }

    pub fn is_subgame(&self) -> bool {
        matches!(self, Handle::SubGame(_))
    }

    pub fn addr(&self) -> String {
        match self {
            Handle::Transactor(h) => h.addr.clone(),
            Handle::Validator(h) => h.addr.clone(),
            Handle::SubGame(h) => h.addr.clone(),
        }
    }

    pub fn bundle_addr(&self) -> String {
        match self {
            Handle::Transactor(h) => h.bundle_addr.clone(),
            Handle::Validator(h) => h.bundle_addr.clone(),
            Handle::SubGame(h) => h.bundle_addr.clone(),
        }
    }

    /// Wait handle until it's shutted down.  A
    /// [SignalFrame::RemoveGame] will be sent through `signal_tx`.
    pub fn wait(&mut self, signal_tx: mpsc::Sender<SignalFrame>) -> JoinHandle<CloseReason> {
        let handles = match self {
            Handle::Transactor(ref mut x) => &mut x.handles,
            Handle::Validator(ref mut x) => &mut x.handles,
            Handle::SubGame(ref mut x) => &mut x.handles,
        };
        if handles.is_empty() {
            panic!("Some where else is waiting");
        }
        let handles = std::mem::take(handles);
        let addr = self.addr();
        tokio::spawn(async move {
            let mut close_reason = CloseReason::Complete;
            for h in handles.into_iter() {
                let cr = h.wait().await;
                if let CloseReason::Fault(_) = cr {
                    close_reason = cr
                }
            }
            if let Err(e) = signal_tx
                .send(SignalFrame::RemoveGame { game_addr: addr })
                .await
            {
                error!("Failed to send RemoveGame signal due to {}", e);
            }
            close_reason
        })
    }
}
