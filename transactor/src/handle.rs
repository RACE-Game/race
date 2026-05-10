mod subgame;
mod transactor;
mod validator;

use std::sync::Arc;

use race_transactor_frames::{BridgeToParent, SignalFrame};
use race_transactor_components::{
    Broadcaster, CloseReason, EventBus, WrappedStorage, WrappedTransport,
};
use race_core::error::{Error, Result};
use race_core::types::ServerAccount;
use race_core::checkpoint::ContextCheckpoint;
use race_encryptor::Encryptor;
use race_env::TransactorConfig;
use tokio::sync::mpsc::unbounded_channel;
use subgame::SubGameHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, warn};
use transactor::TransactorHandle;
use validator::ValidatorHandle;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
    SubGame(SubGameHandle),
}

impl Handle {
    pub async fn try_new_transactor(
        game_addr: String,
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
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
        game_addr: String,
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
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
        checkpoint: ContextCheckpoint,
        bridge_to_parent: BridgeToParent,
        transport: Arc<WrappedTransport>,
        encryptor: Arc<Encryptor>,
        storage: Arc<WrappedStorage>,
        server_account: &ServerAccount,
        config: &TransactorConfig,
    ) -> Result<Self> {
        Ok(Self::SubGame(
            SubGameHandle::try_new(
                checkpoint,
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

    pub fn bundle_key(&self) -> String {
        match self {
            Handle::Transactor(h) => h.bundle_key.clone(),
            Handle::Validator(h) => h.bundle_key.clone(),
            Handle::SubGame(h) => h.bundle_key.clone(),
        }
    }

    /// Wait handle until it's shutted down.  A
    /// [SignalFrame::RemoveGame] will be sent through `signal_tx`.
    pub fn wait(&mut self, signal_tx: mpsc::Sender<SignalFrame>) -> JoinHandle<CloseReason> {
        let (handles, addr) = match self {
            Handle::Transactor(ref mut x) => (&mut x.handles, x.addr.clone()),
            Handle::Validator(ref mut x) => (&mut x.handles, x.addr.clone()),
            Handle::SubGame(ref mut x) => (&mut x.handles, x.addr.clone()),
        };
        if handles.is_empty() {
            panic!("Some where else is waiting");
        }
        let handles = std::mem::take(handles);
        tokio::spawn(async move {
            let (exit_tx, mut exit_rx) = unbounded_channel();
            let expected_components = handles.len();

            for h in handles.into_iter() {
                let exit_tx = exit_tx.clone();
                let component_id = h.id.clone();
                tokio::spawn(async move {
                    let close_reason = h.wait().await;
                    let _ = exit_tx.send((component_id, close_reason));
                });
            }
            drop(exit_tx);

            let mut close_reason = CloseReason::Complete;

            if let Some((component_id, first_reason)) = exit_rx.recv().await {
                match &first_reason {
                    CloseReason::Fault(err) => {
                        warn!(
                            "Game {} component {} exited with fault: {}.",
                            addr,
                            component_id,
                            err
                        );
                        close_reason = first_reason.clone();
                    }
                    CloseReason::Complete => {
                        warn!(
                            "Game {} component {} exited unexpectedly with Complete.",
                            addr,
                            component_id,
                        );
                    }
                }

                for _ in 1..expected_components {
                    let Some((component_id, reason)) = exit_rx.recv().await else {
                        break;
                    };
                    match &reason {
                        CloseReason::Fault(err) => {
                            warn!(
                                "Game {} component {} exited with fault during shutdown: {}",
                                addr,
                                component_id,
                                err
                            );
                            close_reason = reason;
                        }
                        CloseReason::Complete => {}
                    }
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
