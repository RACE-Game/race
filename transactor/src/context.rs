use crate::component::{Broadcaster, Component, EventBus, EventLoop, GameSynchronizer, Submitter};
use race_core::types::{AttachGameParams, EventFrame};
use race_facade::FacadeTransport;
use std::collections::HashMap;
use std::sync::Arc;

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
}

impl Handle {
    pub async fn new(addr: &str, chain: &str) -> Result<Self> {
        let transport = create_transport(chain)?;
        let init_state = transport.get_game_account(addr).await.ok_or(Error::GameAccountNotFound)?;
        let event_bus = EventBus::default();
        let submitter = Submitter::new(transport.clone(), init_state.clone());
        let synchronizer = GameSynchronizer::new(transport.clone(), init_state.clone());

        Ok(Self {
            addr: addr.into(),
            event_bus,
            submitter,
            synchronizer,
        })
    }

    /// Start the handle by starting all its components.
    pub async fn start(&mut self) {
        self.submitter.start();
        self.synchronizer.start();
        self.event_bus.attach(&self.submitter).await;
        self.event_bus.attach(&self.synchronizer).await;
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

    pub async fn send_event(&self, addr: &str, event: EventFrame) -> Result<()> {
        if let Some(handle) = self.handles.get(addr) {
            handle.event_bus.send(event).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}

/// Transactor runtime context
#[derive(Default)]
pub struct Context {
    pub game_manager: GameManager,
}
