use race_api::prelude::InitAccount;
use race_api::effect::Effect;
use race_core::error::Result;
use race_api::event::Event;

pub trait HandlerT: Send + Sync {
    fn handle_event(&mut self, effect: &Effect, event: &Event) -> Result<Effect>;

    fn init_state(&mut self, init_account: &InitAccount) -> Result<Effect>;
}
