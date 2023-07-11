use std::sync::Arc;

use async_trait::async_trait;
use race_core::types::{GameAccount, Settle, SettleOp, SettleParams};

use crate::component::common::{Component, ConsumerPorts, Ports};
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;
use race_core::transport::TransportT;
use std::collections::BTreeMap;

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

    /// This fn takes a vector of `Settle' structs to build a settle
    /// map which stores the squashed settles and a list contains all
    /// eject settles.  These results will later be used to make the
    /// real settles to be submitted.
    pub fn squash(settles: &Vec<Settle>) -> (BTreeMap<String, SettleOp>, Vec<Settle>) {
        let mut settle_map = BTreeMap::<String, SettleOp>::new();
        let mut settle_ejects = Vec::<Settle>::new();
        for settle in settles.iter() {
            match settle.op {
                SettleOp::Add(amt) => {
                    settle_map
                        .entry(settle.addr.clone())
                        .and_modify(|sop| match sop {
                            SettleOp::Add(old_amt) => {
                                let new_amt = *old_amt + amt;
                                *sop = SettleOp::Add(new_amt);
                            }
                            SettleOp::Sub(old_amt) => {
                                if *old_amt >= amt {
                                    let new_amt = *old_amt - amt;
                                    *sop = SettleOp::Sub(new_amt);
                                } else {
                                    let new_amt = amt - *old_amt;
                                    *sop = SettleOp::Add(new_amt);
                                };
                            }
                            SettleOp::Eject => {}
                        })
                        .or_insert(SettleOp::Add(amt));
                }

                SettleOp::Sub(amt) => {
                    settle_map
                        .entry(settle.addr.clone())
                        .and_modify(|sop| match sop {
                            SettleOp::Add(old_amt) => {
                                if *old_amt >= amt {
                                    let new_amt = *old_amt - amt;
                                    *sop = SettleOp::Add(new_amt);
                                } else {
                                    let new_amt = amt - *old_amt;
                                    *sop = SettleOp::Sub(new_amt);
                                };
                            }
                            SettleOp::Sub(old_amt) => {
                                let new_amt = *old_amt + amt;
                                *sop = SettleOp::Sub(new_amt);
                            }
                            SettleOp::Eject => {}
                        })
                        .or_insert(SettleOp::Sub(amt));
                }

                SettleOp::Eject => {
                    settle_ejects.push(settle.clone());
                }
            }
        }
        (settle_map, settle_ejects)
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
                    let (settle_map, mut settle_ejects) = Self::squash(&settles);

                    // Settle `Leave` or `Eject` as soon as we receive it
                    if settles
                        .iter()
                        .any(|Settle { addr: _, op }| matches!(op, SettleOp::Eject))
                        || settle_map.len() >= 2
                    {
                        let mut settles = settle_map
                            .into_iter()
                            .map(|(addr, op)| Settle { addr, op })
                            .collect::<Vec<Settle>>();

                        settles.append(&mut settle_ejects);

                        // The wrapped transport will return only when the transaction succeeds.
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
        let charlie = TestClient::player("charlie");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .add_player(&charlie, 100)
            .build();
        let transport = Arc::new(DummyTransport::default());
        let (submitter, ctx) = Submitter::init(&game_account, transport.clone());

        let settles = vec![
            Settle::sub("alice", 100),
            Settle::add("alice", 50),
            Settle::add("bob", 50),
            Settle::add("bob", 100),
            Settle::sub("bob", 20),
            Settle::eject("charlie"),
        ];

        let (settle_map, _settle_rejects) = Submitter::squash(&settles);
        println!("-- settle map {:?}", settle_map);

        let squashed_settles = settle_map
            .into_iter()
            .map(|(addr, op)| Settle { addr, op })
            .collect::<Vec<Settle>>();
        println!("-- squashed settles {:?}", squashed_settles);

        let event_frame = EventFrame::Settle {
            settles: squashed_settles.clone(),
        };
        let mut handle = submitter.start(ctx);

        handle.send_unchecked(event_frame).await;
        handle.send_unchecked(EventFrame::Shutdown).await;
        handle.wait().await;

        assert_eq!(*transport.get_settles(), squashed_settles);
    }
}
