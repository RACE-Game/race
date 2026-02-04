use std::mem::swap;
use std::collections::HashMap;

use race_api::engine::GameHandler;
use race_api::event::Event;
use race_api::effect::Effect;
use race_api::init_account::InitAccount;
use race_core::context::{EventEffects, GameContext};
use race_core::engine::general_handle_event;
use race_core::error::Result;
use race_encryptor::Encryptor;

// Some event has special handling in event loop.
fn patch_handle_event_effects(context: &mut GameContext, event_effects: &EventEffects) {
    if event_effects.start_game {
        let _ = context.dispatch(Event::GameStart, 0);
    }
}

/// A wrapped handler for testing
/// This handler includes the general event handling, which is necessary for integration test.
pub struct TestHandler<H>
where
    H: GameHandler,
{
    handler: H,
    random_results: HashMap<usize, HashMap<usize, String>>,
}

impl<H: GameHandler> TestHandler<H> {

    pub fn new_with_handler(
        handler: H
    ) -> Self {
        let random_results = HashMap::default();
        Self { handler, random_results }
    }

    pub fn init_state(
        context: &mut GameContext,
        init_account: InitAccount,
    ) -> Result<(Self, EventEffects)> {
        let mut new_context = context.clone();
        let mut effect = Effect::default();
        effect.is_init = true;
        let handler = H::init_state(&mut effect, init_account)?;
        let event_effects = new_context.apply_effect(effect)?;
        let random_results = HashMap::default();
        patch_handle_event_effects(&mut new_context, &event_effects);
        swap(context, &mut new_context);
        Ok((Self { handler, random_results }, event_effects))
    }

    pub fn handle_event(
        &mut self,
        context: &mut GameContext,
        event: &Event,
    ) -> Result<EventEffects> {
        let mut new_context = context.clone();
        let encryptor = Encryptor::default();
        general_handle_event(&mut new_context, event, &encryptor)?;
        let mut effect = new_context.derive_effect();
        // patch the fake random result if we have
        if !self.random_results.is_empty() {
            effect.revealed = self.random_results.clone();
        }
        self.handler.handle_event(&mut effect, event.to_owned())?;
        let event_effects = new_context.apply_effect(effect)?;
        patch_handle_event_effects(&mut new_context, &event_effects);
        swap(context, &mut new_context);
        Ok(event_effects)
    }

    pub fn set_random_result(&mut self, random_id: usize, result: HashMap<usize, String>) {
        self.random_results.insert(random_id, result);
    }

    pub fn state(&self) -> &H {
        &self.handler
    }

    pub fn state_mut(&mut self) -> &mut H {
        &mut self.handler
    }
}
