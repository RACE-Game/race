use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, LocalConnection,
    RemoteConnection, Submitter, Subscriber, WrappedClient, WrappedHandler,
};
use race_core::context::GameContext;
use race_core::encryptor::EncryptorT;
use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::{GameAccount, GameBundle, ServerAccount, ClientMode};
use tracing::info;

pub enum Handle {
    Transactor(TransactorHandle),
    Validator(ValidatorHandle),
}

#[allow(dead_code)]
pub struct TransactorHandle {
    addr: String,
    event_bus: EventBus,
    submitter: Submitter,
    synchronizer: GameSynchronizer,
    broadcaster: Broadcaster,
    client: WrappedClient,
    event_loop: EventLoop,
}

impl TransactorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<dyn EncryptorT>,
        transport: Arc<dyn TransportT>,
    ) -> Result<Self> {
        info!("Use transactor mode");
        let mut game_context = GameContext::try_new(&game_account)?;
        let mut handler =
            WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;
        handler.init_state(&mut game_context, game_account)?;

        let event_bus = EventBus::default();
        let mut broadcaster =
            Broadcaster::new(&game_account, game_context.get_handler_state_json().into());
        let mut event_loop = EventLoop::new(handler, game_context, ClientMode::Transactor);
        let mut submitter = Submitter::new(transport.clone(), game_account.clone());
        let mut synchronizer = GameSynchronizer::new(transport.clone(), game_account.clone());
        let mut connection = LocalConnection::new(encryptor.clone());

        info!("Creating components");
        event_bus.attach(&mut connection).await;
        let mut client = WrappedClient::new(
            server_account,
            game_account,
            transport.clone(),
            encryptor,
            Arc::new(connection),
        );

        info!("Attaching components");
        event_bus.attach(&mut submitter).await;
        event_bus.attach(&mut synchronizer).await;
        event_bus.attach(&mut event_loop).await;
        event_bus.attach(&mut client).await;
        event_bus.attach(&mut broadcaster).await;

        info!("Starting components");
        submitter.start();
        synchronizer.start();
        broadcaster.start();
        client.start();
        event_loop.start();

        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            submitter,
            synchronizer,
            broadcaster,
            client,
            event_loop,
        })
    }
}

#[allow(dead_code)]
pub struct ValidatorHandle {
    addr: String,
    event_bus: EventBus,
    subscriber: Subscriber,
    client: WrappedClient,
    event_loop: EventLoop,
}

impl ValidatorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<dyn EncryptorT>,
        transport: Arc<dyn TransportT>,
    ) -> Result<Self> {
        info!("Use transactor mode");
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
        let mut event_loop = EventLoop::new(handler, game_context, ClientMode::Validator);
        let connection = Arc::new(
            RemoteConnection::try_new(
                &server_account.addr,
                &transactor_account.endpoint,
                encryptor.clone(),
            )
            .await?,
        );
        let mut subscriber = Subscriber::new(game_account, server_account, transport.clone(), connection.clone());
        let mut client = WrappedClient::new(
            server_account,
            game_account,
            transport.clone(),
            encryptor,
            connection,
        );

        info!("Attaching components");
        event_bus.attach(&mut event_loop).await;
        event_bus.attach(&mut client).await;
        event_bus.attach(&mut subscriber).await;

        info!("Starting components");
        client.start();
        subscriber.start();
        event_loop.start();

        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            subscriber,
            client,
            event_loop,
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
}
