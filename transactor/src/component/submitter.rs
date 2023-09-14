use std::sync::Arc;

use async_trait::async_trait;
use race_core::types::{GameAccount, SettleParams};

use crate::component::common::{Component, ConsumerPorts};
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

    async fn run(mut ports: ConsumerPorts, ctx: SubmitterContext) -> CloseReason {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::Settle { settles, transfers } => {
                    let res = ctx
                        .transport
                        .settle_game(SettleParams {
                            addr: ctx.addr.clone(),
                            settles,
                            transfers,
                        })
                        .await;

                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            return CloseReason::Fault(e);
                        }
                    }
                }
                EventFrame::Shutdown => {
                    break;
                }
                _ => (),
            }
        }
        return CloseReason::Complete
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
        let charlie = TestClient::player("charlie");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .add_player(&charlie, 100)
            .build();
        let transport = Arc::new(DummyTransport::default());
        let (submitter, ctx) = Submitter::init(&game_account, transport.clone());

        let settles = vec![
            Settle::sub("alice", 50),
            Settle::add("alice", 20),
            Settle::add("alice", 20),
            Settle::sub("alice", 40),
            Settle::add("bob", 50),
            Settle::sub("bob", 20),
            Settle::sub("bob", 20),
            Settle::sub("bob", 20),
            Settle::add("bob", 30),
            Settle::eject("charlie"),
        ];

        let event_frame = EventFrame::Settle {
            settles: settles.clone(),
            transfers: vec![],
        };
        let mut handle = submitter.start(ctx);

        handle.send_unchecked(event_frame).await;
        handle.send_unchecked(EventFrame::Shutdown).await;
        handle.wait().await;

        assert_eq!(*transport.get_settles(), settles);
    }
}
