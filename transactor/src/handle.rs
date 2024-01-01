use std::sync::Arc;

use crate::component::{
    Broadcaster, CloseReason, Component, EventBridgeParent, EventBus, EventLoop,
    GameSynchronizer, LocalConnection, PortsHandle, RemoteConnection, Submitter, Subscriber, Voter,
    WrappedClient, WrappedHandler, WrappedTransport, EventBridgeChild,
};
use crate::frame::{EventFrame, SignalFrame};
use race_api::error::{Error, Result};
use race_api::prelude::InitAccount;
use race_core::context::GameContext;
use race_core::transport::TransportT;
use race_core::types::{
    ClientMode, GameAccount, GameBundle, QueryMode, ServerAccount, SubGameSpec,
};
use race_encryptor::Encryptor;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
    SubGame(SubGameHandle),
}

#[allow(dead_code)]
pub struct TransactorHandle {
    addr: String,
    handles: Vec<PortsHandle>,
    event_bus: EventBus,
    broadcaster: Broadcaster,
    bridge_parent: EventBridgeParent,
}

impl TransactorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        signal_tx: mpsc::Sender<SignalFrame>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Transactor mode",
            game_account.addr
        );

        let game_context = GameContext::try_new(game_account)?;
        let handler = WrappedHandler::load_by_bundle(bundle_account, encryptor.clone()).await?;

        let event_bus = EventBus::default();

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(game_account.addr.clone());
        let mut broadcaster_handle = broadcaster.start(broadcaster_ctx);

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let (submitter, submitter_ctx) = Submitter::init(game_account, transport.clone());
        let mut submitter_handle = submitter.start(submitter_ctx);

        let (synchronizer, synchronizer_ctx) =
            GameSynchronizer::init(transport.clone(), game_account);

        let mut connection = LocalConnection::new(encryptor.clone());

        event_bus.attach(&mut connection).await;
        let (client, client_ctx) = WrappedClient::init(
            server_account,
            game_account,
            transport.clone(),
            encryptor,
            Arc::new(connection),
        );
        let mut client_handle = client.start(client_ctx);

        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut submitter_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut client_handle).await;

        // Dispatch init state
        let init_account = game_account.derive_init_account();
        info!("InitAccount: {:?}", init_account);
        event_bus.send(EventFrame::InitState { init_account }).await;

        let mut synchronizer_handle = synchronizer.start(synchronizer_ctx);
        event_bus.attach(&mut synchronizer_handle).await;

        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            handles: vec![
                broadcaster_handle,
                submitter_handle,
                event_loop_handle,
                client_handle,
                synchronizer_handle,
            ],
            broadcaster,
            bridge_parent: bridge,
        })
    }
}

#[allow(dead_code)]
pub struct ValidatorHandle {
    addr: String,
    event_bus: EventBus,
    handles: Vec<PortsHandle>,
    bridge_parent: EventBridgeParent,
}

impl ValidatorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<Encryptor>,
        transport: Arc<WrappedTransport>,
        signal_tx: mpsc::Sender<SignalFrame>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Validator mode",
            game_account.addr
        );
        let game_context = GameContext::try_new(game_account)?;
        let handler = WrappedHandler::load_by_bundle(bundle_account, encryptor.clone()).await?;

        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        let transactor_account = transport
            .get_server_account(transactor_addr)
            .await?
            .ok_or(Error::CantFindTransactor)?;

        info!("Creating components");
        let event_bus = EventBus::default();

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Validator);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let connection = Arc::new(
            RemoteConnection::try_new(
                &server_account.addr,
                &transactor_account.endpoint,
                encryptor.clone(),
            )
            .await?,
        );
        let (subscriber, subscriber_context) =
            Subscriber::init(game_account, server_account, connection.clone());
        let mut subscriber_handle = subscriber.start(subscriber_context);

        let (client, client_ctx) = WrappedClient::init(
            server_account,
            game_account,
            transport.clone(),
            encryptor,
            connection,
        );
        let mut client_handle = client.start(client_ctx);

        let (voter, voter_ctx) = Voter::init(game_account, server_account, transport.clone());
        let mut voter_handle = voter.start(voter_ctx);

        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut voter_handle).await;
        event_bus.attach(&mut client_handle).await;

        let init_account = game_account.derive_rollbacked_init_account();
        info!("InitAccount: {:?}", init_account);

        // Dispatch init state
        event_bus.send(EventFrame::InitState { init_account }).await;

        event_bus.attach(&mut subscriber_handle).await;
        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            handles: vec![
                subscriber_handle,
                client_handle,
                event_loop_handle,
                voter_handle,
            ],
            bridge_parent: bridge,
        })
    }
}

#[allow(dead_code)]
pub struct SubGameHandle {
    addr: String,
    event_bus: EventBus,
    handles: Vec<PortsHandle>,
    broadcaster: Broadcaster,
    bridge_child: EventBridgeChild,
}

impl SubGameHandle {
    pub async fn try_new(
        spec: SubGameSpec,
        bridge_parent: EventBridgeParent,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
    ) -> Result<Self> {
        let game_addr = spec.game_addr.clone();
        let sub_id = spec.sub_id.clone();
        let addr = format!("{}:{}", game_addr, sub_id);
        let event_bus = EventBus::default();

        let bundle_account = transport
            .get_game_bundle(&spec.bundle_addr)
            .await?
            .ok_or(Error::GameBundleNotFound)?;

        // Build an InitAccount
        let mut init_account = InitAccount::default();
        init_account.addr = addr.clone();
        init_account.data = spec.init_data.clone();

        let game_context = GameContext::try_new_with_sub_game_spec(spec)?;
        let handler = WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(addr.clone());
        let mut broadcaster_handle = broadcaster.start(broadcaster_ctx);

        let (bridge, bridge_ctx) = bridge_parent.derive_child(sub_id.clone());
        let mut bridge_handle = bridge.start(bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.send(EventFrame::InitState { init_account }).await;

        Ok(Self {
            addr: format!("{}:{}", game_addr, sub_id),
            event_bus,
            handles: vec![broadcaster_handle, bridge_handle, event_loop_handle],
            broadcaster,
            bridge_child: bridge
        })
    }
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
        encryptor: Arc<Encryptor>,
        server_account: &ServerAccount,
        addr: &str,
        signal_tx: mpsc::Sender<SignalFrame>,
    ) -> Result<Self> {
        info!("Try create game handle for {}", addr);
        let mode = QueryMode::Finalized;
        let game_account = transport
            .get_game_account(addr, mode)
            .await?
            .ok_or(Error::GameAccountNotFound)?;

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
                        signal_tx,
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
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
    ) -> Result<Self> {
        let handle = SubGameHandle::try_new(spec, bridge_parent, encryptor, transport).await?;
        Ok(Self::SubGame(handle))
    }

    pub fn broadcaster(&self) -> Result<&Broadcaster> {
        match self {
            Handle::Transactor(h) => Ok(&h.broadcaster),
            Handle::Validator(_) => Err(Error::NotSupportedInValidatorMode),
            Handle::SubGame(_) => Err(Error::NotSupportedInSubGameMode),
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
