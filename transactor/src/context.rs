use crate::handle::Handle;
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::{AttachGameParams, EventFrame, TransactorAccount};
use race_env::Config;
use race_transport::create_transport;
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

async fn get_transactor_account(config: &Config) -> Result<TransactorAccount> {
    if let Some(ref transactor_config) = config.transactor {
        let transport = create_transport(config, &transactor_config.chain)?;
        transport.get_transactor_account(&transactor_config.address).await.ok_or(Error::InvalidTransactorAddress)
    } else {
        Err(Error::TransactorConfigMissing)
    }
}


/// Transactor runtime context
pub struct ApplicationContext {
    pub config: Config,
    pub account: TransactorAccount,
    pub games: HashMap<String, Handle>,
}

impl ApplicationContext {
    pub async fn new(config: Config) -> Self {
        let account = get_transactor_account(&config).await.expect("Failed to read on-chain transactor account");

        Self {
            config,
            account,
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
