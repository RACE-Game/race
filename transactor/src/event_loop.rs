use crate::game::{GameHandle, MessageFrame};
use jsonrpsee::core::DeserializeOwned;
use race_core::context::GameContext;
use race_core::engine::GameHandler;
use race_core::types::Address;
use serde::Serialize;
use tokio;
use tokio::sync::mpsc;

pub struct EventLoop<H, E>
where
    H: GameHandler<E> + Serialize,
    E: DeserializeOwned,
{
    _event_type: std::marker::PhantomData<E>,
    context: GameContext,
    handler: H,
    receiver: mpsc::Receiver<MessageFrame>,
}

impl<H, E> EventLoop<H, E>
where
    H: GameHandler<E> + Serialize,
    E: DeserializeOwned,
{
    pub fn new(addr: Address, handler: H, receiver: mpsc::Receiver<MessageFrame>) -> Self {
        let context = GameContext::new(addr);

        Self {
            _event_type: std::marker::PhantomData::default(),
            context,
            handler,
            receiver,
        }
    }

    pub async fn start(&mut self) {
        loop {
            let message_frame = self.receiver.recv().await;
            if let Some(message_frame) = message_frame {
                match message_frame {
                    MessageFrame::SendEvent(event_frame, tx) => {
                        let r = self.handler
                            .handle_raw_game_event(&mut self.context, &event_frame.data);
                        let r = serde_json::to_string(&r).unwrap();
                        tx.send(r).unwrap();
                    }
                    MessageFrame::GetState(tx) => {
                        let r = serde_json::to_string(&self.handler).unwrap();
                        tx.send(r).unwrap();
                    }
                }
            } else {
                // Stop the event loop if the channel is closed
                println!("Event loop quit");
                break;
            }
        }
    }
}
