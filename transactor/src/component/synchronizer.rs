use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{oneshot, watch},
    time::sleep,
};

use race_core::transport::TransportT;
use race_core::types::{EventFrame, GameAccount};

use crate::component::{
    event_bus::{player_joined, CloseReason},
    traits::{Attachable, Component, Named},
};

pub(crate) struct GameSynchronizerContext {
    output_tx: watch::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    transport: Arc<dyn TransportT>,
    init_state: GameAccount,
}

/// A component that reads the on-chain states and feed the system.
/// To construct a synchronizer, a chain adapter is required.
pub struct GameSynchronizer {
    output_rx: watch::Receiver<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<GameSynchronizerContext>,
}

impl Named for GameSynchronizer {
    fn name<'a>(&self) -> &'a str {
        "GameSynchronzier"
    }
}

impl Attachable for GameSynchronizer {
    fn input(&self) -> Option<tokio::sync::mpsc::Sender<EventFrame>> {
        None
    }

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        Some(self.output_rx.clone())
    }
}

impl Component<GameSynchronizerContext> for GameSynchronizer {
    fn run(&mut self, ctx: GameSynchronizerContext) {
        tokio::spawn(async move {
            let init_state = ctx.init_state;

            let mut access_serial = init_state.access_serial;
            let mut curr_players = init_state.players;

            loop {
                let state = ctx.transport.get_game_account(&init_state.addr).await;
                if let Some(state) = state {
                    if access_serial < state.access_serial {
                        let event = player_joined(init_state.addr.to_owned(), &curr_players, &state.players);
                        if ctx.output_tx.send(event).is_err() {
                            ctx.closed_tx.send(CloseReason::Complete).unwrap();
                            break;
                        }
                        curr_players = state.players;
                        access_serial = state.access_serial;
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
        let (output_tx, output_rx) = watch::channel(EventFrame::Empty);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(GameSynchronizerContext {
            output_tx,
            closed_tx,
            transport,
            init_state,
        });
        Self {
            output_rx,
            closed_rx,
            ctx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use race_core::types::{GameAccount, Player};
    use race_transport::dummy::DummyTransport;

    #[tokio::test]
    async fn test_sync_state() {
        let transport = Arc::new(DummyTransport::default());
        let p = Some(Player::new("Alice", 5000));
        let ga_0 = GameAccount {
            addr: DummyTransport::mock_game_account_addr(),
            bundle_addr: DummyTransport::mock_game_bundle_addr(),
            ..Default::default()
        };
        let ga_1 = GameAccount {
            addr: DummyTransport::mock_game_account_addr(),
            bundle_addr: DummyTransport::mock_game_bundle_addr(),
            access_serial: 1,
            players: vec![p.clone()],
            ..Default::default()
        };

        transport.simulate_states(vec![ga_1]);
        let mut synchronizer = GameSynchronizer::new(transport.clone(), ga_0);
        synchronizer.start();
        let output = &mut synchronizer.output_rx;
        output.changed().await.unwrap();
        assert_eq!(*output.borrow(), EventFrame::PlayerJoined { addr: DummyTransport::mock_game_account_addr(), players: vec![p] });
    }
}
