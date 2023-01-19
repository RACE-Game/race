use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use race_core::connection::ConnectionT;

pub struct Connection {
    rpc_client: HttpClient,
}

impl ConnectionT for Connection {
    type Transport = HttpClient;

    fn transport(&self) -> &Self::Transport {
        &self.rpc_client
    }
}

impl Connection {
    pub async fn new(endpoint: &str) -> Self {
        Self {
            rpc_client: HttpClientBuilder::default()
                .build(format!("ws://{}", endpoint))
                .unwrap(),
        }
    }
}
