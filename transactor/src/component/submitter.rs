use std::sync::Arc;

use async_trait::async_trait;
use race_core::types::{GameAccount, SettleParams};

use crate::component::common::{Component, ConsumerPorts, Ports};
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;
use race_core::transport::TransportT;

pub struct SubmitterContext {
    addr: String,
    transport: Arc<dyn TransportT>,
}

pub struct Submitter {}

impl Submitter {
    pub fn init(
        game_account: &GameAccount,
        transport: Arc<dyn TransportT>,
    ) -> (Self, SubmitterContext) {
        (
            Self {},
            SubmitterContext {
                addr: game_account.addr.clone(),
                transport,
            },
        )
    }
}

#[async_trait]
impl Component<ConsumerPorts, SubmitterContext> for Submitter {
    fn name(&self) -> &str {
        "Submitter"
    }

    async fn run(mut ports: ConsumerPorts, ctx: SubmitterContext) {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::Settle { settles } => {
                    // The wrapped transport will only return when the transaction is succeed.
                    // So here we assume the settle version is updated.
                    // The new settle_version equals to the old plus 1;
                    let res = ctx
                        .transport
                        .settle_game(SettleParams {
                            addr: ctx.addr.clone(),
                            settles,
                        })
                        .await;

                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            ports.close(CloseReason::Fault(e));
                            return;
                        }
                    }
                }
                EventFrame::Shutdown => {
                    break;
                }
                _ => (),
            }
        }
        ports.close(CloseReason::Complete);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use race_core::types::Settle;
    use race_test::*;

    #[tokio::test]
    async fn test_submit_settle() {
        let alice = TestClient::player("alice");
        let bob = TestClient::player("bob");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .build();
        let transport = Arc::new(DummyTransport::default());
        let (submitter, ctx) = Submitter::init(&game_account, transport.clone());
        let settles = vec![Settle::add("Alice", 100)];
        let event_frame = EventFrame::Settle {
            settles: settles.clone(),
        };
        let mut handle = submitter.start(ctx);

        handle.send_unchecked(event_frame).await;
        handle.send_unchecked(EventFrame::Shutdown).await;
        handle.wait().await;

        assert_eq!(*transport.get_settles(), settles);
    }
}
