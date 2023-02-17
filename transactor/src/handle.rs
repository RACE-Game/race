use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, LocalConnection, PortsHandle,
    RemoteConnection, Submitter, Subscriber, Voter, WrappedClient, WrappedHandler,
};
use race_core::context::GameContext;
use race_core::encryptor::EncryptorT;
use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, GameBundle, ServerAccount};
use tokio::task::JoinHandle;
use tracing::info;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
}

#[allow(dead_code)]
pub struct TransactorHandle {
    addr: String,
    handles: Vec<PortsHandle>,
    event_bus: EventBus,
    broadcaster: Broadcaster,
}

impl TransactorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<dyn EncryptorT>,
        transport: Arc<dyn TransportT>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Transactor mode",
            game_account.addr
        );

        let mut game_context = GameContext::try_new(&game_account)?;
        let mut handler =
            WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;
        handler.init_state(&mut game_context, game_account)?;

        let event_bus = EventBus::default();

        let (broadcaster, broadcaster_ctx) =
            Broadcaster::init(&game_account, game_context.get_handler_state_json().into());
        let mut broadcaster_handle = broadcaster.start(broadcaster_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let (submitter, submitter_ctx) = Submitter::init(&game_account, transport.clone());
        let mut submitter_handle = submitter.start(submitter_ctx);

        let (synchronizer, synchronizer_ctx) =
            GameSynchronizer::init(transport.clone(), &game_account);
        let mut synchronizer_handle = synchronizer.start(synchronizer_ctx);

        let mut connection = LocalConnection::new(encryptor.clone());

        event_bus.attach(&mut connection).await;
        let (client, client_ctx) = WrappedClient::init(
            &server_account,
            &game_account,
            transport.clone(),
            encryptor,
            Arc::new(connection),
        );
        let mut client_handle = client.start(client_ctx);

        info!("Attaching components");
        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut submitter_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut client_handle).await;
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
        })
    }
}

#[allow(dead_code)]
pub struct ValidatorHandle {
    addr: String,
    event_bus: EventBus,
    handles: Vec<PortsHandle>,
}

impl ValidatorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<dyn EncryptorT>,
        transport: Arc<dyn TransportT>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Validator mode",
            game_account.addr
        );
        let mut game_context = GameContext::try_new(&game_account)?;
        let mut handler =
            WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;
        handler.init_state(&mut game_context, game_account)?;
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        let transactor_account = transport
            .get_server_account(&transactor_addr)
            .await
            .ok_or(Error::CantFindTransactor)?;

        info!("Creating components");
        let event_bus = EventBus::default();

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let connection = Arc::new(
            RemoteConnection::try_new(
                &server_account.addr,
                &transactor_account.endpoint,
                encryptor.clone(),
            )
            .await?,
        );
        // let mut subscriber = Subscriber::new(game_account, server_account, connection.clone());
        let (subscriber, subscriber_context) =
            Subscriber::init(game_account, server_account, connection.clone());
        let mut subscriber_handle = subscriber.start(subscriber_context);

        let (client, client_ctx) = WrappedClient::init(
            &server_account,
            &game_account,
            transport.clone(),
            encryptor,
            connection,
        );
        let mut client_handle = client.start(client_ctx);

        let (voter, voter_ctx) = Voter::init(game_account, server_account, transport.clone());
        let mut voter_handle = voter.start(voter_ctx);

        info!("Attaching components");
        event_bus.attach(&mut subscriber_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut voter_handle).await;
        event_bus.attach(&mut client_handle).await;

        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            handles: vec![
                subscriber_handle,
                client_handle,
                event_loop_handle,
                voter_handle,
            ],
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
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        server_account: &ServerAccount,
        addr: &str,
    ) -> Result<Self> {
        info!("Try create game handle for {}", addr);
        let game_account = transport
            .get_game_account(addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;

        if game_account.transactor_addr.is_none() {
            return Err(Error::GameNotServed);
        }

        // Query the game bundle
        info!("Query game bundle: {}", game_account.bundle_addr);
        let game_bundle = transport
            .get_game_bundle(&game_account.bundle_addr)
            .await
            .ok_or(Error::GameBundleNotFound)?;

        if game_account.transactor_addr.as_ref() == Some(&server_account.addr) {
            Ok(Self::Transactor(
                TransactorHandle::try_new(
                    &game_account,
                    server_account,
                    &game_bundle,
                    encryptor.clone(),
                    transport.clone(),
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
                )
                .await?,
            ))
        }
    }

    pub fn broadcaster(&self) -> Result<&Broadcaster> {
        match self {
            Handle::Transactor(h) => Ok(&h.broadcaster),
            Handle::Validator(_) => Err(Error::NotSupportedInValidatorMode),
        }
    }

    pub fn event_bus(&self) -> &EventBus {
        match self {
            Handle::Transactor(h) => &h.event_bus,
            Handle::Validator(h) => &h.event_bus,
        }
    }

    pub fn wait(&mut self) -> JoinHandle<()> {
        let handles = match self {
            Handle::Transactor(ref mut x) => &mut x.handles,
            Handle::Validator(ref mut x) => &mut x.handles,
        };
        if handles.is_empty() {
            panic!("Some where else is waiting");
        }
        let handles = std::mem::replace(handles, vec![]);
        tokio::spawn(async move {
            for mut h in handles.into_iter() {
                h.wait().await;
            }
        })
    }
}
