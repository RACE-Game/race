use race_api::prelude::InitAccount;
use race_core::context::{EventEffects, GameContext};
use race_core::error::Result;
use race_api::event::Event;

pub trait HandlerT: Send + Sync {
    fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<EventEffects>;

    fn init_state(&mut self, context: &mut GameContext, init_account: &InitAccount) -> Result<EventEffects>;
}
