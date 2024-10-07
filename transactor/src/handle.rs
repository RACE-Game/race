mod subgame;
mod transactor;
mod validator;

use std::sync::Arc;

use crate::component::{Broadcaster, CloseReason, EventBridgeParent, EventBus, WrappedStorage, WrappedTransport};
use crate::frame::SignalFrame;
use race_core::checkpoint::Checkpoint;
use race_core::storage::StorageT;
use race_encryptor::Encryptor;
use race_api::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::{GetCheckpointParams, QueryMode, ServerAccount, SubGameSpec};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;
use subgame::SubGameHandle;
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
        let mode = QueryMode::Finalized;
        let mut game_account = transport
            .get_game_account(addr, mode)
            .await?
            .ok_or(Error::GameAccountNotFound)?;

        let checkpoint_offchain = storage.get_checkpoint(GetCheckpointParams {
            game_addr: addr.to_owned(),
            settle_version: game_account.settle_version,
        }).await?;
        let checkpoint = Checkpoint::new_from_parts(checkpoint_offchain, game_account.checkpoint_onchain.clone());
        game_account.set_checkpoint(checkpoint);

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
        spec: SubGameSpec,
        bridge_parent: EventBridgeParent,
        server_account: &ServerAccount,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        debug_mode: bool,
    ) -> Result<Self> {
        let handle =
            SubGameHandle::try_new(spec, bridge_parent, server_account, encryptor, transport, debug_mode)
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

    pub fn event_parent_owned(&self) -> Result<EventBridgeParent> {
        match self {
            Handle::Transactor(h) => Ok(h.bridge_parent.to_owned()),
            Handle::Validator(h) => Ok(h.bridge_parent.to_owned()),
            Handle::SubGame(_) => Err(Error::NotSupportedInSubGameMode),
        }
    }

    pub fn event_bus(&self) -> &EventBus {
        match self {
            Handle::Transactor(h) => &h.event_bus,
            Handle::Validator(h) => &h.event_bus,
            Handle::SubGame(h) => &h.event_bus,
        }
    }

    pub fn wait(&mut self) -> JoinHandle<CloseReason> {
        let handles = match self {
            Handle::Transactor(ref mut x) => &mut x.handles,
            Handle::Validator(ref mut x) => &mut x.handles,
            Handle::SubGame(ref mut x) => &mut x.handles,
        };
        if handles.is_empty() {
            panic!("Some where else is waiting");
        }
        let handles = std::mem::take(handles);
        tokio::spawn(async move {
            let mut close_reason = CloseReason::Complete;
            for h in handles.into_iter() {
                let cr = h.wait().await;
                if let CloseReason::Fault(_) = cr {
                    close_reason = cr
                }
            }
            close_reason
        })
    }
}
