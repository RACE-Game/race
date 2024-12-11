//! The component to send transactions to refund invalid deposits.

use std::sync::Arc;

use async_trait::async_trait;
use race_core::types::{GameAccount, RejectDepositsParams};
use tracing::{error, info};

use crate::component::common::Component;
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;
use race_core::transport::TransportT;

use super::common::PipelinePorts;
use super::ComponentEnv;

pub struct RefunderContext {
    addr: String,
    transport: Arc<dyn TransportT>,
}

pub struct Refunder {}

impl Refunder {
    pub fn init(
        game_account: &GameAccount,
        transport: Arc<dyn TransportT>,
    ) -> (Self, RefunderContext) {
        (
            Self {},
            RefunderContext {
                addr: game_account.addr.clone(),
                transport,
            },
        )
    }
}

#[async_trait]
impl Component<PipelinePorts, RefunderContext> for Refunder {
    fn name() -> &'static str {
        "Refunder"
    }

    async fn run(mut ports: PipelinePorts, ctx: RefunderContext, env: ComponentEnv) -> CloseReason {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::RejectDeposits { reject_deposits } => {
                    if let Err(e) = ctx
                        .transport
                        .reject_deposits(RejectDepositsParams {
                            addr: ctx.addr.clone(),
                            reject_deposits,
                        })
                        .await
                    {
                        error!("{} Error in rejecting deposit", e.to_string());
                    }
                }

                EventFrame::Shutdown => {
                    info!("{} Stopped", env.log_prefix);
                    break;
                }

                _ => (),
            }
        }

        CloseReason::Complete
    }
}
