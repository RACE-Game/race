use crate::frame::EventFrame;
use crate::handle::Handle;
use race_core::event::Event;
use race_core::types::{AttachGameParams, TransactorAccount};
use race_env::Config;
use race_transport::create_transport;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use race_core::error::{Error, Result};

async fn get_transactor_account(config: &Config) -> Result<TransactorAccount> {
    if let Some(ref transactor_config) = config.transactor {
        let transport = create_transport(config, &transactor_config.chain)?;
        transport
            .get_transactor_account(&transactor_config.address)
            .await
            .ok_or(Error::InvalidTransactorAddress)
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
        let account = get_transactor_account(&config)
            .await
            .expect("Failed to read on-chain transactor account");

        Self {
            config,
            account,
            games: HashMap::default(),
        }
    }

    pub async fn start_game(&mut self, params: AttachGameParams) -> Result<()> {
        match self.games.entry(params.addr) {
            Entry::Occupied(_) => Ok(()),
            Entry::Vacant(e) => {
                let mut handle =
                    Handle::new(&self.config, &self.account, e.key(), &params.chain).await?;
                handle.start().await;
                e.insert(handle);
                Ok(())
            }
        }
    }

    pub fn get_game(&self, addr: &str) -> Option<&Handle> {
        self.games.get(addr)
    }

    pub async fn send_event(&self, addr: &str, event: Event) -> Result<()> {
        if let Some(handle) = self.games.get(addr) {
            let event_frame = EventFrame::SendEvent {
                event,
            };
            handle.event_bus.send(event_frame).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}
