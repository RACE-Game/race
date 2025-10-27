use std::time::Duration;

use race_core::{context::GameContext, types::ClientMode};
use race_api::event::Event;
use tokio::select;
use tracing::info;

use race_transactor_frames::EventFrame;
use crate::{common::PipelinePorts, utils::current_timestamp};

pub fn log_execution_context(ctx: &GameContext, evt: &Event) {
    info!("Execution context");
    info!("===== State =====");
    info!("{:?}", ctx.get_handler_state_raw());
    info!("===== Event =====");
    info!("{:?}", evt);
    info!("=================");
}

/// Take the event from clients or the pending dispatched event.
/// Transactor retrieves events from both dispatching event and
/// ports, while Validator retrieves events from only ports.
pub async fn read_event(
    ports: &mut PipelinePorts,
    game_context: &mut GameContext,
    mode: ClientMode,
) -> Option<EventFrame> {
    let timestamp = current_timestamp();
    if mode != ClientMode::Transactor {
        ports.recv().await
    } else if let Some(dispatch) = game_context.get_dispatch() {
        // If already passed
        if dispatch.timeout <= timestamp {
            let event = dispatch.event.clone();
            game_context.cancel_dispatch();
            return Some(EventFrame::SendServerEvent { event, timestamp });
        }
        let to = tokio::time::sleep(Duration::from_millis(dispatch.timeout - timestamp));
        select! {
            ef = ports.recv() => {
                ef
            }
            _ = to => {
                let event = dispatch.event.clone();
                let timestamp = dispatch.timeout;
                game_context.cancel_dispatch();
                Some(EventFrame::SendServerEvent { event, timestamp })
            }
        }
    } else {
        ports.recv().await
    }
}
