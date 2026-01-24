//! This component read the raw Sync event and update the encryptor to include their credentials.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use async_trait::async_trait;
use race_transactor_frames::EventFrame;
use race_core::credentials::Credentials;
use race_core::transport::TransportT;
use race_core::encryptor::EncryptorT;
use race_core::node::Node;
use race_core::types::ClientMode;
use tracing::{info, error};
use borsh::BorshDeserialize;
use super::{common::PipelinePorts, ComponentEnv};
use crate::Component;
use crate::CloseReason;

pub async fn maybe_fetch_server_credentials(
    server_addr: &str,
    cached_server_credentials: &mut HashMap<String, Credentials>,
    transport: Arc<dyn TransportT>,
    env: &ComponentEnv,
) -> Credentials {
    if let Some(credentials) = cached_server_credentials.get(server_addr) {
        return credentials.clone();
    } else {
        loop {
            if let Ok(Some(profile)) = transport.get_server_account(&server_addr).await {
                info!("{} Load server credentials: {}", env.log_prefix, server_addr);
                let credentials = Credentials::try_from_slice(&profile.credentials).expect("Failed to deserialize Credentials");
                cached_server_credentials.insert(server_addr.to_string(), credentials.clone());
                return credentials;
            } else {
                error!("Failed to fetch server profile for {}, will retry.", server_addr);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn maybe_fetch_player_credentials(
    player_addr: &str,
    cached_player_credentials: &mut HashMap<String, Credentials>,
    transport: Arc<dyn TransportT>,
    env: &ComponentEnv,
) -> Credentials {
    if let Some(credentials) = cached_player_credentials.get(player_addr) {
        return credentials.clone();
    } else {
        loop {
            if let Ok(Some(profile)) = transport.get_player_profile(player_addr).await {
                info!("{} Load server credentials: {}", env.log_prefix, player_addr);

                let credentials = Credentials::try_from_slice(&profile.credentials).expect("Failed to deserialize Credentials");
                cached_player_credentials.insert(player_addr.to_string(), credentials.clone());
                return credentials;
            } else {
                error!("Failed to fetch player profile for {}, will retry.", player_addr);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn maybe_fetch_node_credentials(
    node: &Node,
    cached_player_credentials: &mut HashMap<String, Credentials>,
    cached_server_credentials: &mut HashMap<String, Credentials>,
    transport: Arc<dyn TransportT>,
    env: &ComponentEnv,
) -> Credentials {
    match node.mode {
        ClientMode::Player => {
            maybe_fetch_player_credentials(&node.addr, cached_player_credentials, transport, env).await
        }
        _ => {
            maybe_fetch_server_credentials(&node.addr, cached_server_credentials, transport, env).await
        }
    }
}

pub struct CredentialConsolidatorContext {
    transport: Arc<dyn TransportT>,
    encryptor: Arc<dyn EncryptorT>,
    #[allow(unused)]
    game_addr: String,
}

pub struct CredentialConsolidator {}

impl CredentialConsolidator {
    pub fn init(
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        game_addr: &str,
    ) -> (Self, CredentialConsolidatorContext) {
        (
            Self {},
            CredentialConsolidatorContext {
                transport,
                encryptor,
                game_addr: game_addr.to_string(),
            },
        )
    }
}

#[async_trait]
impl Component<PipelinePorts, CredentialConsolidatorContext> for CredentialConsolidator {
    fn name() -> &'static str {
        "Credential Consolidator"
    }

    async fn run(
        mut ports: PipelinePorts,
        ctx: CredentialConsolidatorContext,
        env: ComponentEnv,
    ) -> CloseReason {
        let CredentialConsolidatorContext {
            transport, encryptor, game_addr: _
        } = ctx;

        let mut cached_player_credentials = HashMap::<String, Credentials>::default();
        let mut cached_server_credentials = HashMap::<String, Credentials>::default();

        while let Some(event_frame) = ports.recv().await {
            match event_frame {
                EventFrame::Shutdown => {
                    return CloseReason::Complete;
                }
                EventFrame::RecoverCheckpoint {
                    checkpoint
                }=> {
                    for n in checkpoint.shared_data.nodes.iter() {
                        let credentials = maybe_fetch_node_credentials(
                            n, &mut cached_player_credentials, &mut cached_server_credentials, transport.clone(), &env
                        ).await;

                        if let Err(e) = encryptor.import_credentials(&n.addr, credentials) {
                            return CloseReason::Fault(e.into());
                        }
                    }

                    ports.send(EventFrame::RecoverCheckpointWithCredentials {
                        checkpoint
                    }).await;
                }
                EventFrame::Sync {
                    new_players,
                    new_servers,
                    new_deposits,
                    access_version,
                    transactor_addr,
                } => {
                    for p in new_players.iter() {
                        let credentials = maybe_fetch_player_credentials(&p.addr, &mut cached_player_credentials, transport.clone(), &env).await;
                        if let Err(e) = encryptor.import_credentials(&p.addr, credentials) {
                            return CloseReason::Fault(e.into());
                        }
                    }

                    for s in new_servers.iter() {
                        let credentials = maybe_fetch_server_credentials(&s.addr, &mut cached_server_credentials, transport.clone(), &env).await;
                        if let Err(e) = encryptor.import_credentials(&s.addr, credentials) {
                            return CloseReason::Fault(e.into());
                        }
                    }

                    ports.send(EventFrame::SyncWithCredentials {
                        new_players,
                        new_servers,
                        new_deposits,
                        access_version,
                        transactor_addr,
                    }).await;
                }
                _ => {},
            }
        }

        CloseReason::Complete
    }
}
