use std::sync::Arc;

use async_trait::async_trait;
use race_core::error::Error;
use tokio::select;
use tokio_stream::StreamExt;
use std::collections::HashMap;
use borsh::BorshDeserialize;
use race_core::credentials::Credentials;

use race_transactor_frames::{EventFrame, PlayerJoinSync, ServerJoinSync};
use race_core::{
    transport::TransportT,
    types::{GameAccount, PlayerDeposit, PlayerJoin, ServerJoin},
};
use tracing::{error, info, warn};

use crate::{common::Component, event_bus::CloseReason};

use super::{common::PipelinePorts, ComponentEnv};

pub struct GameSynchronizerContext {
    transport: Arc<dyn TransportT>,
    access_version: u64,
    game_addr: String,
}

async fn fetch_server_with_credentials(
    server_join: ServerJoin,
    cached_server_credentials: &mut HashMap<String, Credentials>,
    transport: Arc<dyn TransportT>,
) -> ServerJoinSync {
    if let Some(credentials) = cached_server_credentials.get(&server_join.addr) {
        return ServerJoinSync {
            addr: server_join.addr,
            endpoint: server_join.endpoint,
            access_version: server_join.access_version,
            credentials: credentials.clone(),
        }
    } else {
        loop {
            if let Ok(Some(profile)) = transport.get_player_profile(&server_join.addr).await {
                let credentials = Credentials::try_from_slice(&profile.credentials).expect("Failed to deserialize Credentials");
                cached_server_credentials.insert(server_join.addr.clone(), credentials.clone());
                return ServerJoinSync {
                    addr: server_join.addr,
                    endpoint: server_join.endpoint,
                    access_version: server_join.access_version,
                    credentials,
                }
            } else {
                error!("Failed to fetch server profile for {}, will retry.", server_join.addr);
            }
        }
    }
}

async fn fetch_player_with_credentials(
    player_join: PlayerJoin,
    cached_player_credentials: &mut HashMap<String, Credentials>,
    transport: Arc<dyn TransportT>,
) -> PlayerJoinSync {
    if let Some(credentials) = cached_player_credentials.get(&player_join.addr) {
        return PlayerJoinSync {
            addr: player_join.addr,
            position: player_join.position,
            access_version: player_join.access_version,
            credentials: credentials.clone(),
        }
    } else {
        loop {
            if let Ok(Some(profile)) = transport.get_player_profile(&player_join.addr).await {
                let credentials = Credentials::try_from_slice(&profile.credentials).expect("Failed to deserialize Credentials");
                cached_player_credentials.insert(player_join.addr.clone(), credentials.clone());
                return PlayerJoinSync {
                    addr: player_join.addr,
                    position: player_join.position,
                    access_version: player_join.access_version,
                    credentials,
                }
            } else {
                error!("Failed to fetch player profile for {}, will retry.", player_join.addr);
            }
        }
    }
}

async fn maybe_send_sync(
    transport: Arc<dyn TransportT>,
    cached_player_credentials: &mut HashMap<String, Credentials>,
    cached_server_credentials: &mut HashMap<String, Credentials>,
    prev_access_version: u64,
    game_account: GameAccount,
    ports: &mut PipelinePorts,
    env: &ComponentEnv,
) -> (u64, Option<CloseReason>) {
    let GameAccount {
        players,
        servers,
        deposits,
        access_version,
        transactor_addr,
        ..
    } = game_account;

    // Drop duplicated updates
    if access_version <= prev_access_version {
        return (prev_access_version, None);
    }

    info!(
        "{} Synchronizer found new game state, access_version = {}, settle_version = {}",
        env.log_prefix, game_account.access_version, game_account.settle_version,
    );

    let mut new_players: Vec<PlayerJoinSync> = Vec::with_capacity(players.len());
    let mut new_servers: Vec<ServerJoinSync> = Vec::with_capacity(servers.len());

    for p in players {
        if p.access_version > prev_access_version { // We care only new players
            let player_sync = fetch_player_with_credentials(p, cached_player_credentials, transport.clone()).await;
            new_players.push(player_sync);
        }
    }

    for s in servers {
        if s.access_version > prev_access_version { // We care only new servers
            let server_sync = fetch_server_with_credentials(s, cached_server_credentials, transport.clone()).await;
            new_servers.push(server_sync);
        }
    }

    let new_deposits: Vec<PlayerDeposit> = deposits
        .into_iter()
        .filter(|d| d.access_version > prev_access_version)
        .collect();

    if !new_players.is_empty() {
        info!(
            "{} New players: {:?}",
            env.log_prefix,
            new_players
                .iter()
                .map(|p| format!("{}@{}#A{}", p.addr, p.position, p.access_version))
                .collect::<Vec<String>>()
                .join(",")
        );
    }

    if !new_deposits.is_empty() {
        info!("{} New deposits: {:?}", env.log_prefix,
            new_deposits
            .iter()
            .map(|d| format!("{}${}#A{}#S{}", d.addr, d.amount, d.access_version, d.settle_version))
            .collect::<Vec<String>>()
            .join(","));
    }

    if !new_servers.is_empty() {
        info!("{} New servers: {:?}", env.log_prefix,
            new_servers
            .iter()
            .map(|s| format!("{}#A{}", s.addr, s.access_version))
            .collect::<Vec<String>>()
            .join(","));
    }

    if !new_players.is_empty() || !new_servers.is_empty() || !new_deposits.is_empty() {
        let frame = EventFrame::Sync {
            new_players,
            new_servers,
            new_deposits,
            // TODO: Handle transactor addr change
            transactor_addr: transactor_addr.unwrap().clone(),
            access_version,
        };

        // When other channels are closed
        if ports.try_send(frame).await.is_err() {
            return (prev_access_version, Some(CloseReason::Complete));
        }
    }

    (access_version, None)
}

/// A component that reads the on-chain states and feeds the system.
/// To construct a synchronizer, a chain adapter is required.
pub struct GameSynchronizer {}

impl GameSynchronizer {
    pub fn init(
        transport: Arc<dyn TransportT>,
        game_account: &GameAccount,
    ) -> (Self, GameSynchronizerContext) {
        (
            Self {},
            GameSynchronizerContext {
                transport,
                access_version: game_account.access_version,
                game_addr: game_account.addr.clone(),
            },
        )
    }
}

#[allow(unused_assignments)]
#[async_trait]
impl Component<PipelinePorts, GameSynchronizerContext> for GameSynchronizer {
    fn name() -> &'static str {
        "Game Synchronizer"
    }

    async fn run(
        mut ports: PipelinePorts,
        ctx: GameSynchronizerContext,
        env: ComponentEnv,
    ) -> CloseReason {
        let mut cached_player_credentials = HashMap::<String, Credentials>::default();
        let mut cached_server_credentials = HashMap::<String, Credentials>::default();

        let mut prev_access_version = ctx.access_version;

        let mut sub = match ctx.transport.subscribe_game_account(&ctx.game_addr).await {
            Ok(sub) => sub,
            Err(e) => {
                warn!(
                    "{} Synchronizer failed to subscribe game account updates: {}",
                    env.log_prefix,
                    e.to_string()
                );
                return CloseReason::Fault(Error::GameAccountNotFound);
            }
        };

        loop {
            select! {
                event_frame = ports.recv() => {
                    match event_frame {
                        Some(EventFrame::Shutdown) => {
                            break;
                        }
                        _ => ()
                    }
                }

                sub_item = sub.next() => {

                    // The retry mechanism is implemented in `WrappedTransport`.  An Ok(Err(..))
                    // means the transport has gave up on reading game state.  The Ok(None) stands
                    // for the end of the stream, which is supposed to be sent after an error.  In
                    // both cases, we shutdown the game by sending a Shutdown frame.
                    match sub_item {
                        Some(Ok(game_account)) => {
                            let (new_access_version, close_reason) = maybe_send_sync(
                                ctx.transport.clone(),
                                &mut cached_player_credentials,
                                &mut cached_server_credentials,
                                prev_access_version,
                                game_account,
                                &mut ports,
                                &env
                            ).await;

                            if let Some(close_reason) = close_reason {
                                return close_reason;
                            }
                            prev_access_version = new_access_version;
                        }
                        Some(Err(e)) => {
                            error!("{} Synchronizer encountered an error: {}",
                                env.log_prefix, e.to_string());
                            ports.send(EventFrame::Shutdown).await;
                            return CloseReason::Fault(e);
                        }
                        None => {
                            error!("{} Synchronizer quit due to subscription closed",
                                env.log_prefix);
                            ports.send(EventFrame::Shutdown).await;
                            return CloseReason::Complete;
                        }
                    }
                }
            }
        }

        return CloseReason::Complete;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use race_test::prelude::*;

    #[tokio::test]
    async fn test_sync_state() {
        let transport = Arc::new(DummyTransport::default());
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let mut foo = TestClient::transactor("foo");
        let mut bar = TestClient::validator("bar");
        let ga_0 = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .set_transactor(&mut foo)
            .build();
        let ga_1 = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .set_transactor(&mut foo)
            .add_player(&mut bob, 100)
            .add_validator(&mut bar)
            .build();

        println!("ga_0: {:?}", ga_0);
        println!("ga_1: {:?}", ga_1);

        // Use vec to simulate game accounts
        transport.simulate_states(vec![ga_1.clone(), ga_0.clone(), ga_1.clone()]);
        let (synchronizer, ctx) = GameSynchronizer::init(transport.clone(), &ga_0);
        let mut handle = synchronizer.start("synchronizer", ctx);
        let frame = handle.recv_unchecked().await.unwrap();

        let expected_new_players = vec![PlayerJoin {
            addr: "bob".into(),
            position: 1,
            access_version: 3,
            verify_key: "".into(),
        }];
        let expected_new_servers = vec![ServerJoin {
            addr: "bar".into(),
            endpoint: "".into(),
            access_version: 4,
            verify_key: "".into(),
        }];
        let expected_transactor_addr = "foo".to_string();
        let expected_access_version = 4;

        if let EventFrame::Sync {
            new_players,
            new_servers,
            transactor_addr,
            access_version,
            ..
        } = frame
        {
            assert_eq!(new_players, expected_new_players);
            assert_eq!(new_servers, expected_new_servers);
            assert_eq!(access_version, expected_access_version);
            assert_eq!(transactor_addr, expected_transactor_addr);
        }
    }
}
