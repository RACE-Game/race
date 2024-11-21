mod subgame;
mod transactor;
mod validator;

use std::sync::Arc;

use crate::component::{
    BridgeToParent, Broadcaster, CloseReason, EventBus, WrappedStorage, WrappedTransport
};
use crate::frame::SignalFrame;
use race_core::context::SubGameInit;
use race_core::error::{Error, Result};
use race_core::storage::StorageT;
use race_core::transport::TransportT;
use race_core::types::{GetCheckpointParams, ServerAccount};
use race_encryptor::Encryptor;
use subgame::SubGameHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};
use transactor::TransactorHandle;
use validator::ValidatorHandle;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
    SubGame(SubGameHandle),
}

/// The handle to the components set of a game.
///
/// # Transactor and Validator
/// `TransactorHandle` will be created when current node is the transactor.
/// Otherwise, `ValidatorHandle` will be created instead.
///
/// # Upgrade
/// TBD
impl Handle {
    /// Create game handle.
    pub async fn try_new(
        transport: Arc<WrappedTransport>,
        storage: Arc<WrappedStorage>,
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        addr: &str,
        signal_tx: mpsc::Sender<SignalFrame>,
        debug_mode: bool,
    ) -> Result<Self> {
        info!("Try create game handle for {}", addr);
        let game_account = transport
            .get_game_account(addr)
            .await?
            .ok_or(Error::GameAccountNotFound)?;

        let checkpoint_offchain = storage
            .get_checkpoint(GetCheckpointParams {
                game_addr: addr.to_owned(),
                settle_version: game_account.settle_version,
            })
            .await?;

        if let Some(ref transactor_addr) = game_account.transactor_addr {
            info!("Current transactor: {}", transactor_addr);
            // Query the game bundle
            info!("Query game bundle: {}", game_account.bundle_addr);
            let game_bundle = transport
                .get_game_bundle(&game_account.bundle_addr)
                .await?
                .ok_or(Error::GameBundleNotFound)?;

            if transactor_addr.eq(&server_account.addr) {
                Ok(Self::Transactor(
                    TransactorHandle::try_new(
                        &game_account,
                        checkpoint_offchain,
                        server_account,
                        &game_bundle,
                        encryptor.clone(),
                        transport.clone(),
                        storage.clone(),
                        signal_tx,
                        debug_mode,
                    )
                    .await?,
                ))
            } else {
                Ok(Self::Validator(
                    ValidatorHandle::try_new(
                        &game_account,
                        checkpoint_offchain,
                        server_account,
                        &game_bundle,
                        encryptor.clone(),
                        transport.clone(),
                        signal_tx,
                        debug_mode,
                    )
                    .await?,
                ))
            }
        } else {
            Err(Error::GameNotServed)
        }
    }

    pub async fn try_new_sub_game_handle(
        sub_game_init: SubGameInit,
        bridge_to_parent: BridgeToParent,
        server_account: &ServerAccount,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        debug_mode: bool,
    ) -> Result<Self> {
        let handle = SubGameHandle::try_new(
            sub_game_init,
            bridge_to_parent,
            server_account,
            encryptor,
            transport,
            debug_mode,
        )
        .await?;
        Ok(Self::SubGame(handle))
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
            if let Err(e) = signal_tx.send(SignalFrame::RemoveGame { game_addr: addr }).await {
                error!("Failed to send RemoveGame signal due to {}", e);
            }
            close_reason
        })
    }
}
