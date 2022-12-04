use crate::component::{Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, Submitter, WrappedHandler};
use race_core::context::GameContext;
use race_core::event::Event;
use race_core::types::{AttachGameParams, EventFrame};
use race_facade::FacadeTransport;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use race_core::error::{Error, Result};
use race_core::transport::TransportT;

/// Create transport based on chain name.
pub fn create_transport(chain: &str) -> Result<Arc<dyn TransportT>> {
    match chain {
        "facade" => Ok(Arc::new(FacadeTransport::default())),
        _ => Err(Error::InvalidChainName),
    }
}

pub struct Handle {
    pub addr: String,
    pub event_bus: EventBus,
    pub submitter: Submitter,
    pub synchronizer: GameSynchronizer,
    pub broadcaster: Broadcaster,
    pub event_loop: EventLoop,
}

impl Handle {
    pub async fn new(addr: &str, chain: &str) -> Result<Self> {
        let transport = create_transport(chain)?;
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
        let event_loop = EventLoop::new(handler, game_account);

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

#[derive(Default)]
pub struct GameManager {
    pub handles: HashMap<String, Handle>,
}

impl GameManager {
    pub async fn start_game(&mut self, params: AttachGameParams) -> Result<()> {
        if !self.handles.contains_key(&params.addr) {
            let mut handle = Handle::new(&params.addr, &params.chain).await?;
            handle.start().await;
            self.handles.insert(params.addr, handle);
        }
        Ok(())
    }

    pub fn get_game(&self, addr: &str) -> Option<&Handle> {
        self.handles.get(addr)
    }

    pub async fn send_event(&self, addr: &str, event: Event) -> Result<()> {
        if let Some(handle) = self.handles.get(addr) {
            let event_frame = EventFrame::SendEvent { addr: addr.to_owned(), event };
            handle.event_bus.send(event_frame).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}

/// Transactor runtime context
pub struct ApplicationContext {
    pub game_manager: GameManager,
    pub broadcast_tx: broadcast::Sender<EventFrame>,
}

impl Default for ApplicationContext {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(16);
        Self {
            broadcast_tx: tx,
            game_manager: GameManager::default(),
        }
    }
}
