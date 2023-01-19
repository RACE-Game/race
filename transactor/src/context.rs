use crate::component::WrappedTransport;
use crate::frame::EventFrame;
use crate::handle::Handle;
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::{AttachGameParams, ServerAccount};
use race_env::{Config, TransactorConfig};
use race_transport::ChainType;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

/// Transactor runtime context
pub struct ApplicationContext {
    pub config: TransactorConfig,
    pub chain: ChainType,
    pub account: ServerAccount,
    pub transport: Arc<dyn TransportT>,
    pub games: HashMap<String, Handle>,
}

impl ApplicationContext {
    pub async fn try_new(config: Config) -> Result<Self> {
        let transport = Arc::new(WrappedTransport::try_new(&config).await?);

        let transactor_config = config
            .transactor
            .ok_or(Error::TransactorConfigMissing)?;

        let chain: ChainType = transactor_config.chain.as_str().try_into()?;

        let account = transport
            .get_server_account(&transactor_config.address)
            .await
            .ok_or(Error::InvalidTransactorAddress)?;

        Ok(Self {
            config: transactor_config,
            chain,
            account,
            transport,
            games: HashMap::default(),
        })
    }

    pub async fn start_game(&mut self, params: AttachGameParams) -> Result<()> {
        println!("Start game from address: {:?}", params.addr);
        match self.games.entry(params.addr) {
            Entry::Occupied(_) => Ok(()),
            Entry::Vacant(e) => {
                let mut handle =
                    Handle::try_new(self.transport.clone(), &self.account, e.key()).await?;
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
            let event_frame = EventFrame::SendEvent { event };
            handle.event_bus.send(event_frame).await;
            Ok(())
        } else {
            Err(Error::GameNotLoaded)
        }
    }
}
