use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::time::sleep;

use crate::frame::EventFrame;
use race_core::types::GameAccount;
use race_core::{
    transport::TransportT,
    types::{PlayerJoin, ServerJoin},
};
use tracing::info;

use crate::component::{
    common::{Component, Ports, ProducerPorts},
    event_bus::CloseReason,
};

pub struct GameSynchronizerContext {
    transport: Arc<dyn TransportT>,
    access_version: u64,
    game_addr: String,
}

/// A component that reads the on-chain states and feed the system.
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

#[async_trait]
impl Component<ProducerPorts, GameSynchronizerContext> for GameSynchronizer {
    fn name(&self) -> &str {
        "Game synchronizer"
    }

    async fn run(ports: ProducerPorts, ctx: GameSynchronizerContext) {
        let mut access_version = ctx.access_version;

        loop {
            let state = ctx.transport.get_game_account(&ctx.game_addr).await;
            if let Some(state) = state {
                if access_version < state.access_version {
                    info!("Synchronizer get new state: {:?}", state);
                    let GameAccount {
                        access_version: av,
                        players,
                        deposits: _,
                        servers,
                        transactor_addr,
                        ..
                    } = state;
                    let new_players: Vec<PlayerJoin> = players
                        .into_iter()
                        .filter(|p| p.access_version > access_version)
                        .collect();
                    let new_servers: Vec<ServerJoin> = servers
                        .into_iter()
                        .filter(|s| s.access_version > access_version)
                        .collect();

                    if !new_players.is_empty() || !new_servers.is_empty() {
                        let frame = EventFrame::Sync {
                            new_players,
                            new_servers,
                            // TODO: Handle transactor addr change
                            transactor_addr: transactor_addr.unwrap().clone(),
                            access_version: state.access_version,
                        };
                        ports.send(frame).await;
                        ports.close(CloseReason::Complete);
                        break;
                    }
                    access_version = av;
                } else {
                    sleep(Duration::from_secs(1)).await;
                }
            } else {
                break;
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
        let ga_0 = TestGameAccountBuilder::default()
            .add_players(1)
            .add_servers(1)
            .build();
        let ga_1 = TestGameAccountBuilder::from_account(&ga_0)
            .add_players(1)
            .add_servers(1)
            .build();
        println!("ga_0: {:?}", ga_0);
        println!("ga_1: {:?}", ga_1);

        let access_version = ga_1.access_version;
        transport.simulate_states(vec![ga_1]);
        let (synchronizer, ctx) = GameSynchronizer::init(transport.clone(), &ga_0);
        let mut handle = synchronizer.start(ctx);

        assert_eq!(
            handle.recv_unchecked().await,
            Some(EventFrame::Sync {
                new_players: vec![PlayerJoin {
                    addr: PLAYER_ADDRS[1].to_owned(),
                    position: 1,
                    balance: DEFAULT_DEPOSIT_AMOUNT,
                    access_version: 3,
                }],
                new_servers: vec![ServerJoin {
                    addr: SERVER_ADDRS[1].to_owned(),
                    endpoint: "".into(),
                    access_version: 4,
                }],
                access_version,
                transactor_addr: transactor_account_addr(),
            })
        );
    }
}
