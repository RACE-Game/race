use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, LocalConnection,
    RemoteConnection, Submitter, WrappedClient, WrappedHandler,
};
use race_core::connection::ConnectionT;
use race_core::context::GameContext;
use race_core::encryptor::EncryptorT;
use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::ServerAccount;

pub struct Handle {
    pub addr: String,
    pub event_bus: EventBus,
    pub submitter: Submitter,
    pub synchronizer: GameSynchronizer,
    pub broadcaster: Broadcaster,
    pub client: WrappedClient,
    pub event_loop: EventLoop,
}

impl Handle {
    pub async fn try_new(
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        account: &ServerAccount,
        addr: &str,
    ) -> Result<Self> {
        println!("Try create game handle for {:?}", addr);

        let game_account = transport
            .get_game_account(addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;

        // Query the transactor on-chain account.
        // Additionally, we check if the game is served, otherwise there may not be
        // enough transactors for randomization, thus the fairness is
        // not guaranteed.
        let transactor_account = if let Some(ref addr) = game_account.transactor_addr {
            transport
                .get_server_account(addr)
                .await
                .ok_or(Error::CantFindTransactor)?
        } else {
            return Err(Error::GameNotServed);
        };

        // Query the game bundle
        let game_bundle = transport
            .get_game_bundle(&game_account.bundle_addr)
            .await
            .ok_or(Error::GameBundleNotFound)?;

        let mut handler = WrappedHandler::load_by_bundle(&game_bundle, encryptor.clone()).await?;
        let mut game_context = GameContext::new(&game_account)?;

        // Create event bus
        let event_bus = EventBus::default();

        // Create the connection to transactor based on client mode.
        // If this node is the transactor, the connection connects to event bus.
        // Otherwise, it connects to the remote transactor.
        let connection: Arc<dyn ConnectionT> =
            if game_account.transactor_addr.as_ref() == Some(&account.addr) {
                let mut conn = LocalConnection::new(encryptor.clone());
                event_bus.attach(&mut conn).await;
                Arc::new(conn)
            } else {
                Arc::new(RemoteConnection::try_new(
                    &transactor_account.endpoint,
                    encryptor.clone(),
                )?)
            };

        // We should initialize the game state if we are the main transactor
        // Otherwise, the state will be initialized when receiving the first event
        handler.init_state(&mut game_context, &game_account)?;
        let submitter = Submitter::new(transport.clone(), game_account.clone());
        let synchronizer = GameSynchronizer::new(transport.clone(), game_account.clone());
        let broadcaster =
            Broadcaster::new(&game_account, game_context.get_handler_state_json().into());
        let event_loop = EventLoop::new(handler, game_context);
        let client = WrappedClient::new(
            account,
            &game_account,
            transport.clone(),
            connection.clone(),
        );

        Ok(Self {
            addr: addr.into(),
            event_bus,
            submitter,
            synchronizer,
            broadcaster,
            client,
            event_loop,
        })
    }

    /// Start the handle by starting all its components.
    pub async fn start(&mut self) {
        self.submitter.start();
        self.synchronizer.start();
        self.broadcaster.start();
        self.client.start();
        self.event_loop.start();

        self.event_bus.attach(&mut self.submitter).await;
        self.event_bus.attach(&mut self.synchronizer).await;
        self.event_bus.attach(&mut self.event_loop).await;
        self.event_bus.attach(&mut self.client).await;
        self.event_bus.attach(&mut self.broadcaster).await;
    }
}
