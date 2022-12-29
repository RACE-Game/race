use crate::handle::Handle;
use race_core::event::Event;
use race_core::types::{AttachGameParams, EventFrame};
use race_env::Config;
use std::collections::HashMap;
use tokio::sync::broadcast;

use race_core::error::{Error, Result};

#[derive(Default)]
pub struct GameManager {
    pub handles: HashMap<String, Handle>,
}

impl GameManager {
    pub async fn start_game(&mut self, config: &Config, params: AttachGameParams) -> Result<()> {
        if !self.handles.contains_key(&params.addr) {
            let mut handle = Handle::new(config, &params.addr, &params.chain).await?;
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
            let event_frame = EventFrame::SendEvent {
                addr: addr.to_owned(),
                event,
            };
            handle.event_bus.send(event_frame).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}

/// Transactor runtime context
pub struct ApplicationContext {
    pub config: Config,
    pub games: HashMap<String, Handle>,
    pub broadcast_tx: broadcast::Sender<EventFrame>,
}

impl ApplicationContext {
    pub fn new(config: Config) -> Self {
        let (tx, _rx) = broadcast::channel(16);
        Self {
            config,
            broadcast_tx: tx,
            games: HashMap::default(),
        }
    }

    pub async fn start_game(&mut self, params: AttachGameParams) -> Result<()> {
        if !self.games.contains_key(&params.addr) {
            let mut handle = Handle::new(&self.config, &params.addr, &params.chain).await?;
            handle.start().await;
            self.games.insert(params.addr, handle);
        }
        Ok(())
    }

    pub fn get_game(&self, addr: &str) -> Option<&Handle> {
        self.games.get(addr)
    }

    pub async fn send_event(&self, addr: &str, event: Event) -> Result<()> {
        if let Some(handle) = self.games.get(addr) {
            let event_frame = EventFrame::SendEvent {
                addr: addr.to_owned(),
                event,
            };
            handle.event_bus.send(event_frame).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}
