use jsonrpsee::core::client::Client;
use jsonrpsee::wasm_client::WasmClientBuilder;
use race_core::connection::ConnectionT;

pub struct Connection {
    rpc_client: Client,
}

impl ConnectionT for Connection {
    type Transport = Client;

    fn transport(&self) -> &Self::Transport {
        &self.rpc_client
    }
}

impl Connection {
    pub async fn new(endpoint: &str) -> Self {
        Self {
            rpc_client: WasmClientBuilder::default()
                .build(format!("ws://{}", endpoint))
                .await
                .unwrap(),
        }
    }
}
