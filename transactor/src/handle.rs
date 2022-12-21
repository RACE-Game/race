use crate::component::{Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, Submitter, WrappedHandler};
use race_core::context::GameContext;
use race_core::event::Event;
use race_core::types::{AttachGameParams, EventFrame};
use race_env::Config;
use race_transport::create_transport;
use race_core::error::{Error, Result};

pub struct Handle {
    pub addr: String,
    pub event_bus: EventBus,
    pub submitter: Submitter,
    pub synchronizer: GameSynchronizer,
    pub broadcaster: Broadcaster,
    pub event_loop: EventLoop,
}

impl Handle {
    pub async fn new(config: &Config, addr: &str, chain: &str) -> Result<Self> {
        let transport = create_transport(config, chain)?;
        println!("Transport for {:?} created", chain);
        let game_account = transport
            .get_game_account(addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;
        let mut handler = WrappedHandler::load_by_addr(addr, transport.as_ref()).await?;
        let mut game_context = GameContext::new(&game_account);
        handler.init_state(&mut game_context, &game_account);
        let event_bus = EventBus::default();
        let submitter = Submitter::new(transport.clone(), game_account.clone());
        let synchronizer = GameSynchronizer::new(transport.clone(), game_account.clone());
        let broadcaster = Broadcaster::new(&game_account);
        let event_loop = EventLoop::new(handler, game_context);

        Ok(Self {
            addr: addr.into(),
            event_bus,
            submitter,
            synchronizer,
            broadcaster,
            event_loop,
        })
    }

    /// Start the handle by starting all its components.
    pub async fn start(&mut self) {
        self.submitter.start();
        self.synchronizer.start();
        self.broadcaster.start();
        self.event_loop.start();

        self.event_bus.attach(&self.submitter).await;
        self.event_bus.attach(&self.synchronizer).await;
        self.event_bus.attach(&self.event_loop).await;
        self.event_bus.attach(&self.broadcaster).await;
    }
}
