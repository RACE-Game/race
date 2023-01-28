use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

use crate::frame::EventFrame;
use race_core::types::GameAccount;
use race_core::{
    transport::TransportT,
    types::{PlayerJoin, ServerJoin},
};
use tracing::info;

use crate::component::{
    event_bus::CloseReason,
    traits::{Attachable, Component, Named},
};

pub(crate) struct GameSynchronizerContext {
    output_tx: mpsc::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    transport: Arc<dyn TransportT>,
    init_state: GameAccount,
}

/// A component that reads the on-chain states and feed the system.
/// To construct a synchronizer, a chain adapter is required.
pub struct GameSynchronizer {
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<GameSynchronizerContext>,
}

impl Named for GameSynchronizer {
    fn name<'a>(&self) -> &'a str {
        "GameSynchronzier"
    }
}

impl Attachable for GameSynchronizer {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        None
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut ret, &mut self.output_rx);
        ret
    }
}

impl Component<GameSynchronizerContext> for GameSynchronizer {
    fn run(&mut self, ctx: GameSynchronizerContext) {
        tokio::spawn(async move {
            let init_state = ctx.init_state;

            let mut access_version = init_state.access_version;

            loop {
                let state = ctx.transport.get_game_account(&init_state.addr).await;
                if let Some(state) = state {
                    if access_version < state.access_version {
                        info!("Synchronizer get new state: {:?}", state);
                        let GameAccount { access_version: av, players, deposits: _, servers, transactor_addr, .. } = state;
                        let new_players: Vec<PlayerJoin> = players.into_iter().filter(|p| p.access_version > access_version).collect();
                        let new_servers: Vec<ServerJoin> = servers.into_iter().filter(|s| s.access_version > access_version).collect();

                        if !new_players.is_empty() || !new_servers.is_empty() {
                            let frame = EventFrame::Sync {
                                new_players,
                                new_servers,
                                // TODO: Handle transactor addr change
                                transactor_addr: transactor_addr.unwrap().clone(),
                                access_version: state.access_version,
                            };
                            if let Err(_e) = ctx.output_tx.send(frame).await {
                                ctx.closed_tx.send(CloseReason::Complete).unwrap();
                                break;
                            }
                        }
                        access_version = av;
                    } else {
                        sleep(Duration::from_secs(5)).await;
                    }
                } else {
                    break;
                }
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<GameSynchronizerContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl GameSynchronizer {
    pub fn new(transport: Arc<dyn TransportT>, init_state: GameAccount) -> Self {
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(GameSynchronizerContext {
            output_tx,
            closed_tx,
            transport,
            init_state,
        });
        Self {
            output_rx: Some(output_rx),
            closed_rx,
            ctx,
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
        let mut synchronizer = GameSynchronizer::new(transport.clone(), ga_0);
        synchronizer.start();

        assert_eq!(
            synchronizer.output_rx.unwrap().recv().await.unwrap(),
            EventFrame::Sync {
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
            }
        );
    }
}
