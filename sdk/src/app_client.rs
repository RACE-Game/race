//! A common client to use in dapp(native version).

use gloo::utils::format::JsValueSerdeExt;
use js_sys::JSON::{parse, stringify};
use js_sys::{Function, Object, Reflect};
use jsonrpsee::core::client::Subscription;
use race_core::context::GameContext;
use race_core::types::{BroadcastFrame, ExitGameParams, RandomId};
use race_transport::TransportLocalT;
use wasm_bindgen::prelude::*;

use futures::pin_mut;
use futures::stream::StreamExt;
use std::cell::RefCell;
use std::sync::Arc;

use crate::connection::Connection;
use crate::handler::Handler;
use crate::transport::Transport;
use crate::utils::{get_function, rget};
use gloo::console::{debug, error, info, warn};

use crate::error::Result;
use crate::js::{Event as JsEvent, JsGameContext};
use race_client::Client;
use race_core::{
    connection::ConnectionT,
    error::Error,
    event::Event,
    types::{ClientMode, GetStateParams, JoinParams, SubmitEventParams},
};
use race_encryptor::Encryptor;

#[wasm_bindgen]
pub struct AppClient {
    addr: String,
    client: Client,
    handler: Handler,
    transport: Arc<Transport>,
    connection: Arc<Connection>,
    game_context: RefCell<GameContext>,
    init_game_account: RefCell<Option<GameAccount>>,
    event_sub: RefCell<Option<Subscription<BroadcastFrame>>>,
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
        transport: JsValue,
        wallet: JsValue,
        game_addr: &str,
        callback: Function,
    ) -> Result<AppClient> {
        info!("Try init");
        let transport = Arc::new(Transport::new(transport));
        AppClient::try_new(transport, wallet, game_addr, callback).await
    }

    async fn try_new(
        transport: Arc<Transport>,
        wallet: JsValue,
        game_addr: &str,
        callback: Function,
    ) -> Result<Self> {
        info!("Try new");
        let encryptor = Arc::new(Encryptor::default());
        info!("Encryptor created");
        info!(&wallet);
        let player_addr = rget(&wallet, "walletAddr")
            .as_string()
            .unwrap();
        info!("Player addr got");
        let game_account = transport
            .get_game_account(game_addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;
        info!("Game account loaded");
        let game_bundle = transport
            .get_game_bundle(&game_account.bundle_addr)
            .await
            .ok_or(Error::GameBundleNotFound)?;
        info!("Game bundle loaded");
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        info!("Game is served");
        let transactor_account = transport
            .get_server_account(transactor_addr)
            .await
            .ok_or(Error::CantFindTransactor)?;
        info!("Transactor account loaded");
        let connection = Arc::new(
            Connection::try_new(&player_addr, &transactor_account.endpoint, encryptor.clone())
                .await?,
        );
        info!("Connection initialized");
        let client = Client::new(
            player_addr.to_owned(),
            game_addr.to_owned(),
            ClientMode::Player,
            transport.clone(),
            encryptor.clone(),
            connection.clone(),
        );
        info!("Game client created");

        let handler = Handler::from_bundle(game_bundle, encryptor).await?;

        let game_context = RefCell::new(GameContext::try_new(&game_account)?);

        Ok(Self {
            addr: game_addr.to_owned(),
            client,
            transport,
            connection,
            handler,
            game_context,
            init_game_account: RefCell::new(Some(game_account)),
            callback,
            event_sub: RefCell::new(None),
        })
    }

    fn invoke_callback(&self, game_context: &GameContext, event: Option<Event>) -> Result<()> {
        let state = parse(game_context.get_handler_state_raw()).or(Err(Error::JsonParseError))?;

        let context = JsGameContext::from_context(&game_context);
        let event_js: JsValue = if let Some(event) = event {
            let event = JsEvent::from(event);
            event.into()
        } else {
            JsValue::UNDEFINED
        };

        let r = Function::call3(
            &self.callback,
            &JsValue::NULL,
            &JsValue::from_serde(&context).or(Err(Error::JsonParseError))?,
            &state,
            &event_js,
        );
        if let Err(e) = r {
            error!(format!("Callback error, {:?}", e));
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn get_profile(&self, addr: &str) -> Option<JsValue> {
        if let Some(p) = self.transport.get_player_profile(addr).await {
            Some(JsValue::from_serde(&p).unwrap())
        } else {
            None
        }
    }

    #[wasm_bindgen]
    /// Attach to game account on chain and connect to the event
    /// streams.  The event stream will start from a
    /// checkpoint(settle_version).  We will receive event hhistories
    /// once the connection is established.
    pub async fn attach_game(&self) -> Result<()> {
        let mut init_game_account = self.init_game_account.borrow_mut();
        if init_game_account.is_none() {
            return Err(Error::DuplicatedInitialization)?;
        }
        info!("Attach to game");
        self.client.attach_game().await?;
        let settle_version = self.game_context.borrow().get_settle_version();

        debug!(
            "Subscribe event stream, use settle_version = {} as check point",
            settle_version
        );

        let sub = self
            .connection
            .subscribe_events(&self.addr, settle_version)
            .await?;

        pin_mut!(sub);
        debug!("Event stream connected");

        while let Some(Ok(frame)) = sub.next().await {
            match frame {
                BroadcastFrame::Init {
                    access_version,
                    settle_version,
                    ..
                } => {
                    let mut game_context = self.game_context.borrow_mut();
                    let game_account = std::mem::replace(&mut *init_game_account, None)
                        .ok_or(Error::DuplicatedInitialization)?;

                    info!(format!(
                        "Apply checkpoint, access_version = {}, settle_version = {}",
                        access_version, settle_version
                    ));

                    game_context.apply_checkpoint(access_version, settle_version)?;

                    let init_account =
                        InitAccount::new(game_account, access_version, settle_version);
                    match self.handler.init_state(&mut game_context, &init_account) {
                        Ok(_) => {
                            self.invoke_callback(&game_context, None)?;
                        }
                        Err(Error::WasmExecutionError(e)) => {
                            error!(format!("Initiate state error: {:?}", e))
                        }
                        Err(e) => {
                            warn!("Init state failed, {}", e.to_string())
                        }
                    }
                }
                BroadcastFrame::Event {
                    event, timestamp, ..
                } => {
                    let mut game_context = self.game_context.borrow_mut();
                    game_context.set_timestamp(timestamp);
                    match self.handler.handle_event(&mut game_context, &event) {
                        Ok(_) => {
                            self.invoke_callback(&game_context, Some(event))?;
                        }
                        Err(Error::WasmExecutionError(e)) => {
                            error!(format!("Handle event error: {:?}", e))
                        }
                        Err(e) => {
                            warn!(format!("Discard event [{}] due to: [{:?}]", event, e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn submit_event(&self, val: JsValue) -> Result<()> {
        info!(format!("Submit event: {:?}", val));
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
    /// Get all revealed information.  This function contains the
    /// decryption, so it's better to cache the result somewhere.
    pub fn get_revealed(&self, random_id: RandomId) -> Result<JsValue> {
        let context = self.game_context.borrow();
        let decrypted = self.client.decrypt(&context, random_id)?;
        let obj = Object::new();
        for (k, v) in decrypted.iter() {
            Reflect::set(&obj, &(*k as u32).into(), &v.into()).unwrap();
        }
        Ok(JsValue::from(obj))
    }

    #[wasm_bindgen]
    pub async fn answer(&mut self, decision_id: DecisionId, value: String) -> Result<()> {
        self.client.answer(decision_id, value).await?;
        Ok(())
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
    pub async fn join(&self, wallet: JsValue, position: u8, amount: u64) -> Result<()> {
        info!("Join game", &wallet);
        let game_account = self
            .transport
            .get_game_account(&self.addr)
            .await
            .ok_or(Error::GameAccountNotFound)?;
        let count: u8 = game_account.players.len() as _;

        if game_account.max_players <= count {
            return Err(Error::GameIsFull(count as _))?;
        }

        let mut position: Option<u8> = None;
        for i in 0..game_account.max_players {
            if game_account
                .players
                .iter()
                .all(|p| p.position != i as usize)
            {
                position = Some(i as _);
                break;
            }
        }

        let position = position.ok_or(Error::GameIsFull(count as _))?;

        self.transport
            .join(
                &wallet,
                JoinParams {
                    game_addr: self.addr.clone(),
                    amount,
                    access_version: game_account.access_version,
                    position: position as _,
                },
            )
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

    #[wasm_bindgen]
    pub async fn close(&self) -> Result<()> {
        self.exit().await?;
        if let Some(event_sub) = self.event_sub.replace(None) {
            event_sub.unsubscribe().await?;
        }
        info!("App client closed");
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
