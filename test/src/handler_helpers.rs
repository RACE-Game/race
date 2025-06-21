use std::mem::swap;

use race_api::engine::GameHandler;
use race_core::error::Result;
use race_api::event::Event;
use race_core::context::{EventEffects, GameContext};
use race_core::engine::general_handle_event;
use race_encryptor::Encryptor;

use crate::client_helpers::TestClient;


// Some event has special handling in event loop.
fn patch_handle_event_effects(context: &mut GameContext, event_effects: &EventEffects) {
    if event_effects.start_game {
        context.dispatch_safe(Event::GameStart, 0);
    }
}

/// A wrapped handler for testing
/// This handler includes the general event handling, which is necessary for integration test.
pub struct TestHandler<H>
where
    H: GameHandler,
{
    handler: H,
}

impl<H: GameHandler> TestHandler<H> {
    pub fn init_state(context: &mut GameContext) -> Result<(Self, EventEffects)> {
        let mut new_context = context.clone();
        let init_account = new_context.init_account();
        let effect = new_context.derive_effect(true);
        let handler = H::init_state(init_account)?;
        let event_effects = new_context.apply_effect(effect)?;
        patch_handle_event_effects(&mut new_context, &event_effects);
        swap(context, &mut new_context);
        Ok((Self { handler }, event_effects))
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<EventEffects> {
        let mut new_context = context.clone();
        let encryptor = Encryptor::default();
        general_handle_event(&mut new_context, event, &encryptor)?;
        let mut effect = new_context.derive_effect(false);
        self.handler.handle_event(&mut effect, event.to_owned())?;
        let event_effects = new_context.apply_effect(effect)?;
        patch_handle_event_effects(&mut new_context, &event_effects);
        swap(context, &mut new_context);
        Ok(event_effects)
    }

    /// Find the event which is going to be disptached in the context, then process it.
    /// In real cases, the disptached event will be handled by an event loop.
    /// We use this function to simulate such cases, since we don't have an event loop in tests.
    pub fn handle_dispatch_event(&mut self, context: &mut GameContext) -> Result<EventEffects> {
        let evt = context
            .get_dispatch()
            .as_ref()
            .expect("No dispatch event")
            .event
            .clone();
        context.cancel_dispatch();
        println!("* Dispatch event: {}", evt);
        self.handle_event(context, &evt)
    }

    pub fn handle_dispatch_until_no_events(
        &mut self,
        context: &mut GameContext,
        clients: Vec<&mut TestClient>,
    ) -> Result<EventEffects> {
        let evt = context
            .get_dispatch()
            .as_ref()
            .expect("No dispatch event")
            .event
            .clone();
        context.cancel_dispatch();
        println!("* Dispatch event: {}", evt);
        self.handle_until_no_events(context, &evt, clients)
    }

    /// This fn keeps handling events of the following two types, until there is none:
    /// 1. Event dispatched from within the (updated) context: context.dispatch
    /// 2. Event dispatched by clients after they see the updated context
    pub fn handle_until_no_events(
        &mut self,
        context: &mut GameContext,
        event: &Event,
        mut clients: Vec<&mut TestClient>,
    ) -> Result<EventEffects> {
        // 1. Process the `event'(arg) --> context updated
        // 2. context may dispatch --> take care those with timeout == current timestamp
        // 3. iter clients to syn with updated context --> a couple of events
        // 4. handle these client/trans events
        let mut evts: Vec<Event> = vec![event.clone()]; // keep handling events in this vec
        let mut event_effects = EventEffects::default();

        while !evts.is_empty() {
            let evt = &evts[0];
            println!("* Received event: {}", evt);

            event_effects = self.handle_event(context, evt)?;
            if evts.len() == 1 {
                evts.clear();
            } else {
                evts = evts.iter().skip(1).map(|e| e.clone()).collect();
            }
            if let Some(ctx_evt) = context.get_dispatch() {
                if ctx_evt.timeout == context.get_timestamp() {
                    evts.push(ctx_evt.event.clone());
                    context.cancel_dispatch();
                }
            }

            for c in clients.iter_mut() {
                let cli_evts = c.handle_updated_context(context)?;
                evts.extend_from_slice(&cli_evts);
                if event_effects.checkpoint.is_some() {
                    c.flush_secret_state();
                }
            }

            if let Some(dispatch) = context.get_dispatch() {
                println!("* Context dispatch: {:?}", dispatch);
            }
        }
        Ok(event_effects)
    }

    pub fn state(&self) -> &H {
        &self.handler
    }

    pub fn state_mut(&mut self) -> &mut H {
        &mut self.handler
    }
}
