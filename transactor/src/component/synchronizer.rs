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
                sleep(Duration::from_secs(5)).await;
                let state = ctx.transport.get_game_account(&init_state.addr).await;
                if let Some(state) = state {
                    if access_serial < state.access_serial {
                        let event = player_joined(&curr_players, &state.players);
                        if ctx.output_tx.send(event).is_err() {
                            ctx.closed_tx.send(CloseReason::Complete).unwrap();
                            break;
                        }
                        curr_players = state.players;
                    }
                    access_serial = state.access_serial;
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
