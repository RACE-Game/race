//! Synchronize the updates from game account. Currently we use pulls
//! instead of websocket. Firstly we query the state with confirming
//! status, if a new state is found, we swtich to query with finalized
//! status. For confirming query we have 5 seconds as interval, for
//! finalized query, we use 2 seconds as interval.

use std::sync::Arc;

use async_trait::async_trait;
use race_core::error::Error;
use tokio::select;
use tokio_stream::StreamExt;

use crate::frame::EventFrame;
use race_core::{
    transport::TransportT,
    types::{GameAccount, PlayerDeposit, PlayerJoin, ServerJoin},
};
use tracing::{info, warn, error};

use crate::component::{common::Component, event_bus::CloseReason};

use super::{common::PipelinePorts, ComponentEnv};

pub struct GameSynchronizerContext {
    transport: Arc<dyn TransportT>,
    access_version: u64,
    game_addr: String,
}

async fn maybe_send_sync(
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

    let new_players: Vec<PlayerJoin> = players
        .into_iter()
        .filter(|p| p.access_version > prev_access_version)
        .collect();

    let new_servers: Vec<ServerJoin> = servers
        .into_iter()
        .filter(|s| s.access_version > prev_access_version)
        .collect();

    let new_deposits: Vec<PlayerDeposit> = deposits
        .into_iter()
        .filter(|d| d.access_version > prev_access_version)
        .collect();

    if !new_players.is_empty() {
        info!("{} New players: {:?}", env.log_prefix, new_players);
    }

    if !new_deposits.is_empty() {
        info!("{} New deposits: {:?}", env.log_prefix, new_deposits);
    }

    if !new_servers.is_empty() {
        info!("{} New servers: {:?}", env.log_prefix, new_servers);
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
                access_version: game_account
                    .checkpoint_on_chain
                    .as_ref()
                    .and_then(|c| Some(c.access_version))
                    .unwrap_or(0),
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
                            let (new_access_version, close_reason) = maybe_send_sync(prev_access_version, game_account, &mut ports, &env).await;
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

    // async fn run(
    //     ports: ProducerPorts,
    //     ctx: GameSynchronizerContext,
    //     env: ComponentEnv,
    // ) -> CloseReason {
    //     let mut prev_access_version = ctx.access_version;
    //     let mut access_version = ctx.access_version;
    //     let mut mode = ctx.mode;
    //     let mut num_of_retries = 0;

    //     loop {
    //         let state = ctx.transport.get_game_account(&ctx.game_addr, mode).await;

    //         if ports.is_tx_closed() {
    //             return CloseReason::Complete;
    //         }

    //         match mode {
    //             QueryMode::Confirming => {
    //                 if let Ok(Some(state)) = state {
    //                     if access_version < state.access_version {
    //                         info!(
    //                             "{} Synchronizer found confirming state, access_version = {}, settle_version = {}",
    //                             env.log_prefix,
    //                             state.access_version,
    //                             state.settle_version,
    //                         );
    //                         let GameAccount {
    //                             access_version: av,
    //                             players,
    //                             deposits: _,
    //                             ..
    //                         } = state;

    //                         let confirm_players: Vec<ConfirmingPlayer> = players
    //                             .into_iter()
    //                             .filter(|p| p.access_version > access_version)
    //                             .map(Into::into)
    //                             .collect();

    //                         if !confirm_players.is_empty() {
    //                             let tx_state = TxState::PlayerConfirming {
    //                                 confirm_players,
    //                                 access_version: state.access_version,
    //                             };
    //                             let frame = EventFrame::TxState { tx_state };

    //                             // When other channels are closed
    //                             if ports.try_send(frame).await.is_err() {
    //                                 return CloseReason::Complete;
    //                             }
    //                         }
    //                         prev_access_version = access_version;
    //                         access_version = av;
    //                         mode = QueryMode::Finalized;
    //                     } else {
    //                         sleep(Duration::from_secs(5)).await;
    //                     }
    //                 }
    //             }

    //             QueryMode::Finalized => {
    //                 if let Ok(Some(state)) = state {
    //                     let GameAccount {
    //                         access_version: av,
    //                         players,
    //                         deposits: _,
    //                         servers,
    //                         transactor_addr,
    //                         ..
    //                     } = state;

    //                     if access_version <= state.access_version {
    //                         info!(
    //                             "{} Synchronizer found a finalized state, access_version = {}, settle_version = {}",
    //                             env.log_prefix,
    //                             state.access_version,
    //                             state.settle_version,
    //                         );
    //                         let new_players: Vec<PlayerJoin> = players
    //                             .into_iter()
    //                             .filter(|p| p.access_version > prev_access_version)
    //                             .collect();

    //                         let new_servers: Vec<ServerJoin> = servers
    //                             .into_iter()
    //                             .filter(|s| s.access_version > prev_access_version)
    //                             .collect();

    //                         if !new_players.is_empty() || !new_servers.is_empty() {
    //                             let frame = EventFrame::Sync {
    //                                 new_players,
    //                                 new_servers,
    //                                 // TODO: Handle transactor addr change
    //                                 transactor_addr: transactor_addr.unwrap().clone(),
    //                                 access_version: state.access_version,
    //                             };

    //                             // When other channels are closed
    //                             if ports.try_send(frame).await.is_err() {
    //                                 return CloseReason::Complete;
    //                             }
    //                         }
    //                         num_of_retries = 0;
    //                         access_version = av;
    //                         mode = QueryMode::Confirming;
    //                         sleep(Duration::from_secs(5)).await;
    //                     } else if num_of_retries < MAX_RETRIES {
    //                         num_of_retries += 1;
    //                         sleep(Duration::from_secs(2)).await;
    //                     } else {
    //                         // Signal absence of a new game state
    //                         let tx_state = TxState::PlayerConfirmingFailed(access_version);

    //                         let frame = EventFrame::TxState { tx_state };

    //                         // When other channels are closed
    //                         if ports.try_send(frame).await.is_err() {
    //                             return CloseReason::Complete;
    //                         }
    //                         mode = QueryMode::Confirming;
    //                         num_of_retries = 0;
    //                         access_version = prev_access_version;
    //                         sleep(Duration::from_secs(5)).await;
    //                     }
    //                 } else {
    //                     warn!("Game account not found, shutdown synchronizer");
    //                     return CloseReason::Complete;
    //                 }
    //             }
    //         }
    //     }
    // }
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
