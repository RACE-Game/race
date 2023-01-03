use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{oneshot, watch},
    time::sleep,
};

use crate::frame::EventFrame;
use race_core::transport::TransportT;
use race_core::types::GameAccount;

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

            let mut access_version = init_state.access_version;
            let mut curr_players = init_state.players;

            loop {
                let state = ctx.transport.get_game_account(&init_state.addr).await;
                if let Some(state) = state {
                    if access_version < state.access_version {
                        let event = player_joined(
                            init_state.addr.to_owned(),
                            &curr_players,
                            &state.players,
                        );
                        if ctx.output_tx.send(event).is_err() {
                            ctx.closed_tx.send(CloseReason::Complete).unwrap();
                            break;
                        }
                        curr_players = state.players;
                        access_version = state.access_version;
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
    use race_core::types::Player;
    use race_core_test::*;

    #[tokio::test]
    async fn test_sync_state() {
        let transport = Arc::new(DummyTransport::default());
        let p = Player::new("Alice", 5000);
        let ga_0 = game_account_with_empty_data();
        let mut ga_1 = game_account_with_empty_data();
        ga_1.access_version = 1;
        ga_1.players = vec![p.clone()];

        transport.simulate_states(vec![ga_1]);
        let mut synchronizer = GameSynchronizer::new(transport.clone(), ga_0);
        synchronizer.start();
        let output = &mut synchronizer.output_rx;
        output.changed().await.unwrap();
        assert_eq!(
            *output.borrow(),
            EventFrame::PlayerJoined {
                addr: game_account_addr(),
                players: vec![p]
            }
        );
    }
}
