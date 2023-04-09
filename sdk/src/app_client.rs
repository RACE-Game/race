//! A common client to use in dapp(native version).

use gloo::utils::format::JsValueSerdeExt;
use js_sys::JSON::{parse, stringify};
use js_sys::{Function, Object, Reflect};
use race_core::context::GameContext;
use race_core::types::{BroadcastFrame, ExitGameParams, RandomId};
use race_transport::{TransportBuilder, TransportLocalT};
use wasm_bindgen::prelude::*;

use futures::pin_mut;
use futures::stream::StreamExt;
use std::cell::RefCell;
use std::sync::Arc;

use crate::connection::Connection;
use crate::handler::Handler;
use gloo::console::{debug, error, info, warn};

use crate::error::Result;
use crate::js::{Event as JsEvent, JsGameContext};
use race_client::Client;
use race_core::{
    connection::ConnectionT,
    error::Error,
    event::Event,
    transport::TransportT,
    types::{ClientMode, GetStateParams, JoinParams, SubmitEventParams},
};
use race_encryptor::Encryptor;

#[wasm_bindgen]
pub struct AppClient {
    addr: String,
    client: Client,
    handler: Handler,
    transport: Arc<dyn TransportLocalT>,
    connection: Arc<Connection>,
    game_context: RefCell<GameContext>,
    callback: Function,
}

#[wasm_bindgen]
impl AppClient {
    /// Try initialize an app client, which will connect to transactor and blockchain RPC.
    ///
    /// # Arguments
    /// * `chain`, The name of blockchain, currently only `"facade"` is supported.
    /// * `rpc`, The endpoint of blockchain RPC.
    /// * `player_addr`, The address of current player.
    /// * `game_addr`, The address of game to attach.
    /// * `callback`, A JS function: function(addr: String, context: PartialGameContext, state: GameState).
    ///   This function will be called when either game context or game state is updated.
    ///   The `addr` can be one of either the game or its sub game.
    #[wasm_bindgen]
    pub async fn try_init(
        chain: &str,
        rpc: &str,
        player_addr: &str,
        game_addr: &str,
        callback: Function,
    ) -> Result<AppClient> {
        let transport = TransportBuilder::default()
            .try_with_chain(chain)?
            .with_rpc(rpc)
            .build()
            .await?;
        AppClient::try_new(Arc::from(transport), player_addr, game_addr, callback).await
    }

    async fn try_new(
        transport: Arc<dyn TransportLocalT>,
        player_addr: &str,
        game_addr: &str,
        callback: Function,
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

        let connection = Arc::new(
            Connection::try_new(player_addr, &transactor_account.endpoint, encryptor.clone())
                .await?,
        );

        let client = Client::new(
            player_addr.to_owned(),
            game_addr.to_owned(),
            ClientMode::Player,
            transport.clone(),
            encryptor.clone(),
            connection.clone(),
        );

        let handler = Handler::from_bundle(game_bundle, encryptor).await;

        let mut game_context = GameContext::try_new(&game_account)?;

        handler.init_state(&mut game_context, &game_account)?;

        let state_js =
            parse(game_context.get_handler_state_json()).map_err(|_| Error::JsonParseError)?;

        let context = JsGameContext::from_context(&game_context);

        let null = JsValue::null();
        Function::call3(
            &callback,
            &null,
            &JsValue::from_serde(&context).unwrap(),
            &state_js,
            &null,
        )
        .expect("Init callback error");

        let game_context = RefCell::new(game_context);
        Ok(Self {
            addr: game_addr.to_owned(),
            client,
            transport,
            connection,
            handler,
            game_context,
            callback,
        })
    }

    #[wasm_bindgen]
    /// Attach to game account on chain and connect to the event
    /// streams.  The event stream will start from a
    /// checkpoint(settle_version).  We will receive event hhistories
    /// once the connection is established.
    pub async fn attach_game(&self) -> Result<()> {
        info!("Attach to game");
        self.client.attach_game().await?;
        let settle_version = self.game_context.borrow().get_settle_version();
        debug!(format!(
            "Subscribe event stream, use settle_version = {} as check point",
            settle_version
        ));

        let sub = self
            .connection
            .subscribe_events(&self.addr, settle_version)
            .await?;
        pin_mut!(sub);
        debug!("Event stream connected");

        while let Some(frame) = sub.next().await {
            let BroadcastFrame {
                game_addr: _,
                event,
                timestamp,
            } = frame;
            let mut game_context = self.game_context.borrow_mut();
            game_context.set_timestamp(timestamp);
            match self.handler.handle_event(&mut game_context, &event) {
                Ok(_) => {
                    let event = JsEvent::from(event);
                    let state_js = parse(game_context.get_handler_state_json())
                        .map_err(|_| Error::JsonParseError)?;

                    let context = JsGameContext::from_context(&game_context);

                    let null = JsValue::null();
                    let r = Function::call3(
                        &self.callback,
                        &null,
                        &JsValue::from_serde(&context).unwrap(),
                        &state_js,
                        &event.into(),
                    );
                    if let Err(e) = r {
                        error!(format!("Callback error, {:?}", e));
                    }
                }
                Err(e) => {
                    warn!(format!("Discard event [{}] due to: [{:?}]", event, e));
                }
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn submit_event(&self, val: JsValue) -> Result<()> {
        // info!(format!("Submit event: {:?}", val));
        let raw = stringify(&val)
            .or(Err(Error::JsonParseError))?
            .as_string()
            .ok_or(Error::JsonParseError)?;
        let event = Event::Custom {
            sender: self.client.addr.clone(),
            raw,
        };
        self.connection
            .submit_event(&self.addr, SubmitEventParams { event })
            .await?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn get_revealed(&self, random_id: RandomId) -> Result<JsValue> {
        let context = self.game_context.borrow();
        Ok(context.get_revealed(random_id).map(|r| {
            let obj = Object::new();
            for (k, v) in r.iter() {
                Reflect::set(&obj, &(*k as u32).into(), &v.into()).unwrap();
            }
            JsValue::from(obj)
        })?)
    }

    /// Get current game state.
    pub async fn get_state(&self) -> Result<JsValue> {
        let state: String = self
            .connection
            .get_state(&self.addr, GetStateParams {})
            .await?;
        Ok(parse(&state).map_err(|_| Error::JsonParseError)?)
    }

    /// Join the game.
    #[wasm_bindgen]
    pub async fn join(&self, position: u8, amount: u64) -> Result<()> {
        info!("Join game");
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
        info!("Exit game");
        self.connection
            .exit_game(&self.addr, ExitGameParams {})
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
        let client = AppClient::try_init(
            "facade",
            "ws://localhost:12002",
            "Alice",
            "COUNTER_GAME_ADDRESS",
            Function::default(),
        )
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
