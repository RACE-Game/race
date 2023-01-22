//! A common client to use in dapp(native version).

use gloo::utils::format::JsValueSerdeExt;
use js_sys::Function;
use js_sys::JSON::{parse, stringify};
use race_core::types::{BroadcastFrame, ExitGameParams};
use race_transport::TransportBuilder;
use wasm_bindgen::prelude::*;

use futures::pin_mut;
use futures::stream::StreamExt;
use std::sync::Arc;

use crate::connection::Connection;
use crate::handler::Handler;
use gloo::console::{debug, error};

use crate::error::Result;
use race_core::{
    client::Client,
    error::Error,
    event::Event,
    transport::TransportT,
    types::{
        AttachGameParams, ClientMode, GetStateParams, JoinParams, SubmitEventParams,
        SubscribeEventParams,
    },
};
use race_encryptor::Encryptor;

#[wasm_bindgen]
pub struct AppClient {
    addr: String,
    client: Client,
    handler: Handler,
    transport: Arc<dyn TransportT>,
    connection: Connection,
}

#[wasm_bindgen]
impl AppClient {
    #[wasm_bindgen]
    pub async fn try_init(
        chain: &str,
        rpc: &str,
        player_addr: &str,
        game_addr: &str,
    ) -> Result<AppClient> {
        let transport = TransportBuilder::default()
            .try_with_chain(chain)?
            .with_rpc(rpc)
            .build()
            .await?;
        AppClient::try_new(Arc::from(transport), player_addr, game_addr).await
    }

    async fn try_new(
        transport: Arc<dyn TransportT>,
        player_addr: &str,
        game_addr: &str,
    ) -> Result<Self> {
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
            .get_server_account(transactor_addr)
            .await
            .ok_or(Error::CantFindTransactor)?;

        let connection =
            Connection::try_new(&transactor_account.endpoint, encryptor.clone()).await?;

        let client = Client::try_new(
            player_addr.to_owned(),
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

    #[wasm_bindgen]
    /// Attach to game with a callback function.
    /// The callback function will receive ()
    pub async fn attach_game_with_callback(&self, callback: Function) -> Result<()> {
        debug!("Attach to game");
        self.connection
            .attach_game(AttachGameParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to attach to game");
        debug!("Subscribe event stream");
        let sub = self
            .connection
            .subscribe_events(SubscribeEventParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to subscribe to event stream");

        pin_mut!(sub);
        debug!("Event stream connected");
        while let Some(frame) = sub.next().await {
            let BroadcastFrame {
                game_addr,
                event,
                state_json,
            } = frame;
            let event_js = JsValue::from_serde(&event).map_err(|_| Error::JsonParseError)?;
            let state_js = parse(&state_json).map_err(|_| Error::JsonParseError)?;
            let this = JsValue::null();
            let r = Function::call3(
                &callback,
                &this,
                &JsValue::from_str(&game_addr),
                &event_js,
                &state_js,
            );
            if let Err(e) = r {
                error!("Callback error, {}", e);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn attach_game(&self) {
        debug!("Attach to game");
        self.connection
            .attach_game(AttachGameParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to attach to game");
        debug!("Subscribe event stream");
        let sub = self
            .connection
            .subscribe_events(SubscribeEventParams {
                addr: self.addr.clone(),
            })
            .await
            .expect("Failed to subscribe to event stream");

        pin_mut!(sub);
        debug!("Event stream connected");
        while let Some(frame) = sub.next().await {
            match JsValue::from_serde(&frame) {
                Ok(v) => debug!(v),
                Err(e) => error!(e.to_string()),
            }
        }
    }

    #[wasm_bindgen]
    pub async fn submit_event(&self, val: JsValue) -> Result<()> {
        let raw = stringify(&val)
            .or(Err(Error::JsonParseError))?
            .as_string()
            .ok_or(Error::JsonParseError)?;
        let event = Event::Custom {
            sender: self.client.addr.clone(),
            raw,
        };
        self.connection
            .submit_event(SubmitEventParams {
                addr: self.addr.clone(),
                event,
            })
            .await?;
        Ok(())
    }

    /// Get current game state.
    pub async fn get_state(&self) -> Result<JsValue> {
        let state: String = self
            .connection
            .get_state(GetStateParams {
                addr: self.addr.clone(),
            })
            .await?;
        Ok(parse(&state).map_err(|_| Error::JsonParseError)?)
    }

    /// Join the game.
    #[wasm_bindgen]
    pub async fn join(&self, position: u8, amount: u64) -> Result<()> {
        let game_account = self
            .transport
            .get_game_account(&self.addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;

        self.transport
            .join(JoinParams {
                player_addr: self.client.addr.clone(),
                game_addr: self.addr.clone(),
                amount,
                access_version: game_account.access_version,
                position: position as _,
            })
            .await?;

        Ok(())
    }

    #[wasm_bindgen]
    pub async fn exit(&self) -> Result<()> {
        self.connection
            .exit_game(ExitGameParams {
                game_addr: self.addr.clone(),
                player_addr: self.client.addr.clone(),
            })
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use gloo::console::info;
    use serde_json::json;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_client() {
        let client = AppClient::try_init("facade", "ws://localhost:12002", "COUNTER_GAME_ADDRESS")
            .await
            .map_err(JsValue::from)
            .expect("Failed to create client");

        info!("Client created");

        client.attach_game().await;

        // let state = client
        //     .get_state()
        //     .await
        //     .map_err(JsValue::from)
        //     .expect("Failed to get state");
    }
}
