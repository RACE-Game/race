use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, Submitter, WrappedHandler, WrappedTransport, WrappedClient,
};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::TransactorAccount;
use race_env::Config;

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
    pub async fn new(config: &Config, account: &TransactorAccount, addr: &str, chain: &str) -> Result<Self> {
        let transport = Arc::new(WrappedTransport::try_new(config).await?);
        println!("Transport for {:?} created", chain);

        let game_account = transport
            .get_game_account(addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;

        // Ensure the game is served, otherwise there may not be
        // enough transactors for randomization, thus the fairness is
        // not guaranteed
        if game_account.transactor_addr.is_none() {
            return Err(Error::GameNotServed);
        }

        let mut handler = WrappedHandler::load_by_addr(addr, transport.as_ref()).await?;
        let mut game_context = GameContext::new(&game_account)?;

        // We should initialize the game state if we are the main transactor
        // Otherwise, the state will be initialized when receiving the first event
        handler.init_state(&mut game_context, &game_account)?;
        let event_bus = EventBus::default();
        let submitter = Submitter::new(transport.clone(), game_account.clone());
        let synchronizer = GameSynchronizer::new(transport.clone(), game_account.clone());
        let broadcaster = Broadcaster::new(&game_account);
        let event_loop = EventLoop::new(handler, game_context);
        let client = WrappedClient::new(account, &game_account, transport.clone());

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

        self.event_bus.attach(&self.submitter).await;
        self.event_bus.attach(&self.synchronizer).await;
        self.event_bus.attach(&self.event_loop).await;
        self.event_bus.attach(&self.client).await;
        self.event_bus.attach(&self.broadcaster).await;
    }
}
