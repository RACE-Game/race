use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::time::sleep;

use crate::frame::EventFrame;
use race_core::{
    transport::TransportT,
    types::{GameAccount, PlayerJoin, QueryMode, ServerJoin, TxState},
};
use tracing::{info, warn};

use crate::component::{
    common::{Component, ProducerPorts},
    event_bus::CloseReason,
};

pub struct GameSynchronizerContext {
    transport: Arc<dyn TransportT>,
    access_version: u64,
    game_addr: String,
    mode: QueryMode,
}

const MAX_RETRIES: u8 = 10;

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
                mode: QueryMode::Confirming,
            },
        )
    }
}

#[allow(unused_assignments)]
#[async_trait]
impl Component<ProducerPorts, GameSynchronizerContext> for GameSynchronizer {
    fn name(&self) -> &str {
        "Game synchronizer"
    }

    async fn run(ports: ProducerPorts, ctx: GameSynchronizerContext) -> CloseReason {
        let mut prev_access_version = ctx.access_version;
        let mut access_version = ctx.access_version;
        let mut mode = ctx.mode;
        let mut num_of_retries = 0;

        loop {
            let state = ctx
                .transport
                .get_game_account(&ctx.game_addr, mode.clone())
                .await;

            if ports.is_tx_closed() {
                return CloseReason::Complete;
            }

            match mode {
                QueryMode::Confirming => {
                    if let Ok(Some(state)) = state {
                        if access_version < state.access_version {
                            info!(
                                "Synchronizer finds a new access_version = {}",
                                state.access_version
                            );
                            let GameAccount {
                                access_version: av,
                                players,
                                deposits: _,
                                ..
                            } = state;

                            let confirm_players: Vec<PlayerJoin> = players
                                .into_iter()
                                .filter(|p| p.access_version > access_version)
                                .collect();
                            info!("Confirming players number =  {}", confirm_players.len());

                            if !confirm_players.is_empty() {
                                let tx_state = TxState::PlayerConfirming {
                                    confirm_players,
                                    access_version: state.access_version,
                                };
                                let frame = EventFrame::TxState { tx_state };

                                // When other channels are closed
                                if let Err(_) = ports.try_send(frame).await {
                                    return CloseReason::Complete;
                                }
                            }
                            prev_access_version = access_version;
                            access_version = av;
                            mode = QueryMode::Finalized;
                        } else {
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                }

                QueryMode::Finalized => {
                    if let Ok(Some(state)) = state {
                        let GameAccount {
                            access_version: av,
                            players,
                            deposits: _,
                            servers,
                            transactor_addr,
                            ..
                        } = state;

                        if access_version <= state.access_version {
                            info!(
                                "Synchronizer gets a new state, access_version = {}",
                                state.access_version
                            );
                            let new_players: Vec<PlayerJoin> = players
                                .into_iter()
                                .filter(|p| p.access_version > prev_access_version)
                                .collect();

                            let new_servers: Vec<ServerJoin> = servers
                                .into_iter()
                                .filter(|s| s.access_version > prev_access_version)
                                .collect();

                            if !new_players.is_empty() || !new_servers.is_empty() {
                                let frame = EventFrame::Sync {
                                    new_players,
                                    new_servers,
                                    // TODO: Handle transactor addr change
                                    transactor_addr: transactor_addr.unwrap().clone(),
                                    access_version: state.access_version,
                                };

                                // When other channels are closed
                                if let Err(_) = ports.try_send(frame).await {
                                    return CloseReason::Complete;
                                }
                            }
                            num_of_retries = 0;
                            access_version = av;
                            mode = QueryMode::Confirming;
                        } else if num_of_retries < MAX_RETRIES {
                            num_of_retries += 1;
                            sleep(Duration::from_secs(1)).await;
                        } else {
                            // Signal absence of a new game state
                            let tx_state = TxState::PlayerConfirmingFailed(access_version);

                            let frame = EventFrame::TxState { tx_state };

                            // When other channels are closed
                            if let Err(_) = ports.try_send(frame).await {
                                return CloseReason::Complete;
                            }
                            mode = QueryMode::Confirming;
                            num_of_retries = 0;
                            access_version = prev_access_version;
                        }
                    } else {
                        warn!("Game account not found, shutdown synchronizer");
                        return CloseReason::Complete;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use race_test::*;

    #[tokio::test]
    async fn test_sync_state() {
        let transport = Arc::new(DummyTransport::default());
        let alice = TestClient::player("alice");
        let bob = TestClient::player("bob");
        let foo = TestClient::transactor("foo");
        let bar = TestClient::validator("bar");
        let ga_0 = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .set_transactor(&foo)
            .build();
        let ga_1 = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .set_transactor(&foo)
            .add_player(&bob, 100)
            .add_validator(&bar)
            .build();

        println!("ga_0: {:?}", ga_0);
        println!("ga_1: {:?}", ga_1);

        // Use vec to simulate game accounts
        transport.simulate_states(vec![ga_1.clone(), ga_0.clone(), ga_1.clone()]);
        let (synchronizer, ctx) = GameSynchronizer::init(transport.clone(), &ga_0);
        let mut handle = synchronizer.start(ctx);
        let frame = handle.recv_unchecked().await.unwrap();

        let test_confirm_players = vec![PlayerJoin {
            addr: "bob".into(),
            position: 1,
            balance: 100,
            access_version: 3,
            verify_key: "".into(),
        }];

        let test_tx_state = TxState::PlayerConfirming {
            confirm_players: test_confirm_players,
            access_version: 4,
        };

        if let EventFrame::TxState { tx_state } = frame {
            assert_eq!(tx_state, test_tx_state);
        } else {
            panic!("Invalid event frame");
        }
    }
}
