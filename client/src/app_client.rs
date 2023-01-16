//! A common client to use in dapp(native version).


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use std::sync::Arc;

use crate::connection::Connection;
use crate::handler::Handler;

use race_core::{
    client::Client,
    connection::ConnectionT,
    error::{Error, Result},
    event::{CustomEvent, Event},
    transport::TransportT,
    types::{AttachGameParams, ClientMode, SubmitEventParams, SubscribeEventParams},
};
use race_encryptor::Encryptor;

pub struct AppClient {
    addr: String,
    client: Client,
    handler: Handler,
    transport: Arc<dyn TransportT>,
    connection: Connection,
}

impl AppClient {
    pub async fn try_new(transport: Arc<dyn TransportT>, game_addr: &str) -> Result<Self> {

        let encryptor = Arc::new(Encryptor::default());

        let game_account = transport
            .get_game_account(game_addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;

        let game_bundle = transport
            .get_game_bundle(&game_account.bundle_addr)
            .await
            .ok_or(Error::GameBundleNotFound)?;

        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;

        let transactor_account = transport
            .get_transactor_account(transactor_addr)
            .await
            .ok_or(Error::CantFindTransactor)?;

        let connection = Connection::new(&transactor_account.endpoint).await;

        let client = Client::try_new(
            game_addr.into(),
            ClientMode::Player,
            transport.clone(),
            encryptor,
        )?;

        let handler = Handler::from_bundle(game_bundle).await;

        Ok(Self {
            addr: game_addr.to_owned(),
            client,
            transport,
            connection,
            handler,
        })
    }

    /// Start subscription and attach to game.
    pub async fn start(&self) {
        // Attach game
        self.connection
            .attach_game(AttachGameParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to attach to game");

        // Subscribe to event stream
        let mut sub = self
            .connection
            .subscribe(SubscribeEventParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to subscribe to event stream");

        while let Some(frame) = sub.next().await {
            if let Ok(frame) = frame {
                println!("Receive: {:?}", frame);
            } else {
                panic!("Err in broadcast");
            }
        }
    }

    /// Send custom event to transactor.
    pub async fn send_event<E: CustomEvent>(&self, event: E) {
        self.connection
            .submit_event(SubmitEventParams {
                addr: self.addr.clone(),
                event: Event::custom(&self.addr, &event),
            })
            .await
            .expect("Failed to send event");
    }
}

// #[cfg(test)]
// mod tests {

//     use super::*;
//     use race_transport::TransportBuilder;
//     use race_test::{DummyTransport, TEST_GAME_ACCOUNT_ADDR};

//     const BIN_PATH: &str = "../target/wasm32-unknown-unknown/release/race_example_counter.wasm";

//     #[test]
//     fn test_init() {
//         let transport: Arc<dyn TransportT> = Arc::from(DummyTransport::default());
//         let app_client = AppClient::try_new(transport, TEST_GAME_ACCOUNT_ADDR);
//     }
// }
