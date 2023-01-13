//! A common client to use in dapp(native version).

mod handler;

use std::sync::Arc;

use handler::Handler;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use race_core::{
    client::Client,
    connection::Connection,
    event::{CustomEvent, Event},
    transport::TransportT,
    types::{ClientMode, SubmitEventParams, AttachGameParams, SubscribeEventParams},
};
use race_encryptor::Encryptor;
use race_transport::create_transport_for_app;

pub struct AppClient {
    pub addr: String,
    pub chain: String,
    pub client: Client,
    pub handler: Handler,
    pub transport: Arc<dyn TransportT>,
    pub connection: Connection<HttpClient>,
}

impl AppClient {
    pub async fn new(chain: &str, rpc: &str, game_addr: &str) -> Self {
        let transport: Arc<dyn TransportT> =
            Arc::from(create_transport_for_app(chain, rpc).expect("Failed to create transport"));
        let encryptor = Arc::new(Encryptor::default());
        let game_account = transport
            .get_game_account(game_addr)
            .await
            .expect("Failed to load game account");
        let game_bundle = transport
            .get_game_bundle(&game_account.bundle_addr)
            .await
            .expect("Failed to load game bundle");
        if let Some(ref transactor_addr) = game_account.transactor_addr {
            let transactor_account = transport
                .get_transactor_account(transactor_addr)
                .await
                .expect("Failed to load transactor account");
            let endpoint = transactor_account.endpoint.clone();
            let rpc_client = HttpClientBuilder::default()
                .build(&endpoint)
                .expect("Failed to build Rpc client for transactor");
            let connection = Connection::new(endpoint, rpc_client);
            let client = Client::new(game_addr.into(), ClientMode::Player, transport.clone(), encryptor)
                .expect("Failed to create client");
            let handler = Handler::new(game_bundle);
            Self {
                addr: game_addr.to_owned(),
                chain: chain.to_owned(),
                client,
                transport,
                connection,
                handler,
            }
            // let connection = Connection::new()
        } else {
            panic!("Game not served");
        }
    }

    /// Start subscription and attach to game.
    pub async fn start(&self) {
        // Attach game
        self.connection.attach_game(AttachGameParams {
            addr: self.addr.clone(),
            chain: self.chain.clone(),
        }).await.expect("Failed to attach to game");

        // Subscribe to event stream
        let mut sub = self.connection.subscribe(SubscribeEventParams {
            addr: self.addr.clone()
        }).await.expect("Failed to subscribe to event stream");

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
        self.connection.submit_event(SubmitEventParams {
            addr: self.addr.clone(),
            event: Event::custom(&self.addr, &event),
        }).await.expect("Failed to send event");
    }
}
