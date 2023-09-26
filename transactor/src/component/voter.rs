//! The component to make voting transaction.  It happens when
//! transactor is considered dropped off.  A shutdown event will be
//! sent when vote is sent.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use race_api::error::Error;
use race_core::{
    transport::TransportT,
    types::{GameAccount, ServerAccount, VoteParams},
};
use tracing::{info, warn};

use super::common::{Component, PipelinePorts};
use crate::frame::EventFrame;

use super::event_bus::CloseReason;

pub struct VoterContext {
    game_addr: String,
    server_addr: String,
    transport: Arc<dyn TransportT>,
}

pub struct Voter {}

impl Voter {
    pub fn init(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        transport: Arc<dyn TransportT>,
    ) -> (Self, VoterContext) {
        (
            Self {},
            VoterContext {
                game_addr: game_account.addr.clone(),
                server_addr: server_account.addr.clone(),
                transport,
            },
        )
    }
}

#[async_trait]
impl Component<PipelinePorts, VoterContext> for Voter {
    fn name(&self) -> &str {
        "Voter"
    }

    async fn run(mut ports: PipelinePorts, ctx: VoterContext) -> CloseReason {
        while let Some(frame) = ports.recv().await {
            match frame {
                EventFrame::Vote { votee, vote_type } => {
                    warn!("Send vote, votee: {}, type: {:?}", votee, vote_type);
                    let params = VoteParams {
                        game_addr: ctx.game_addr.clone(),
                        vote_type,
                        voter_addr: ctx.server_addr.clone(),
                        votee_addr: votee,
                    };
                    // We keep retrying until success.
                    loop {
                        let r = ctx.transport.vote(params.clone()).await;
                        match r {
                            Ok(_) | Err(Error::DuplicatedVote) => {
                                info!("Vote sent");
                                ports.send(EventFrame::Shutdown).await;
                                break;
                            }
                            Err(e) => {
                                warn!("An error occurred in vote: {:?}, will retry.", e);
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    }
                }
                EventFrame::Shutdown => {
                    warn!("Shutdown voter");
                    break;
                }
                _ => (),
            }
        }
        return CloseReason::Complete
    }
}
