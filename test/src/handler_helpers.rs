use std::mem::swap;

use race_core::context::GameContext;
use race_core::engine::{general_handle_event, general_init_state, GameHandler};
use race_core::error::Result;
use race_core::event::Event;
use race_core::types::GameAccount;

/// A wrapped handler for testing
/// This handler includes the general event handling, which is necessary for integration test.
pub struct TestHandler<H>
where
    H: GameHandler,
{
    handler: H,
}

impl<H: GameHandler> TestHandler<H> {
    pub fn init_state(context: &mut GameContext, init_account: &GameAccount) -> Result<Self> {
        let mut new_context = context.clone();
        general_init_state(&mut new_context, init_account)?;
        let handler = H::init_state(&mut new_context, init_account.clone())?;
        swap(context, &mut new_context);
        Ok(Self { handler })
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, event)?;
        self.handler.handle_event(&mut new_context, event.to_owned())?;
        swap(context, &mut new_context);
        Ok(())
    }

    /// Find the event which is going to be disptached in the context, then process it.
    /// In real case, the disptached event will be handled by event loop.
    /// We use this function to simulate the case, since we don't have an event loop in tests.
    pub fn handle_dispatch_event(&mut self, context: &mut GameContext) -> Result<()> {
        let event = context.get_dispatch().as_ref().expect("No dispatch event").event.clone();
        self.handle_event(context, &event)?;
        Ok(())
    }

    pub fn get_state(&self) -> &H {
        &self.handler
    }
}
