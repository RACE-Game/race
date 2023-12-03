use std::mem::swap;

use race_api::engine::GameHandler;
use race_api::error::Result;
use race_api::event::Event;
use race_api::effect::Effect;
use race_core::context::GameContext;
use race_core::engine::{general_handle_event, general_init_state};
use race_core::types::GameAccount;
use race_encryptor::Encryptor;

use crate::client_helpers::TestClient;

fn parse_effect_checkpoint<H: GameHandler>(effect: &mut Effect) -> Result<()> {
    if effect.is_checkpoint {
        effect.__set_checkpoint_raw(vec![]);
    }
    Ok(())
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
    pub fn init_state(context: &mut GameContext, game_account: &GameAccount) -> Result<Self> {
        let mut new_context = context.clone();
        let init_account = game_account.derive_init_account();
        general_init_state(&mut new_context, &init_account)?;
        let mut effect = new_context.derive_effect();
        let handler = H::init_state(&mut effect, init_account)?;
        context.apply_effect(effect)?;
        swap(context, &mut new_context);
        Ok(Self { handler })
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let mut new_context = context.clone();
        let encryptor = Encryptor::default();
        general_handle_event(&mut new_context, event, &encryptor)?;
        let mut effect = new_context.derive_effect();
        self.handler.handle_event(&mut effect, event.to_owned())?;
        parse_effect_checkpoint::<H>(&mut effect)?;
        new_context.apply_effect(effect)?;
        swap(context, &mut new_context);
        Ok(())
    }

    /// Find the event which is going to be disptached in the context, then process it.
    /// In real cases, the disptached event will be handled by an event loop.
    /// We use this function to simulate such cases, since we don't have an event loop in tests.
    pub fn handle_dispatch_event(&mut self, context: &mut GameContext) -> Result<()> {
        let event = context
            .get_dispatch()
            .as_ref()
            .expect("No dispatch event")
            .event
            .clone();
        self.handle_event(context, &event)?;
        Ok(())
    }

    /// This fn keeps handling events of the following two types, until there is none:
    /// 1. Event dispatched from within the (updated) context: context.dispatch
    /// 2. Event dispatched by clients after they see the updated context
    pub fn handle_until_no_events(
        &mut self,
        context: &mut GameContext,
        event: &Event,
        mut clients: Vec<&mut TestClient>,
    ) -> Result<()> {
        // 1. Process the `event'(arg) --> context updated
        // 2. context may dispatch --> take care those with timeout == current timestamp
        // 3. iter clients to syn with updated context --> a couple of events
        // 4. handle these client/trans events
        let mut evts: Vec<Event> = vec![event.clone()]; // keep handling events in this vec

        while !evts.is_empty() {
            let evt = &evts[0];
            println!("* Received event: {}", evt);

            self.handle_event(context, evt)?;
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
            // Handle events (responses) from Clients/transactor(s) after they see updated ctx
            for i in 0..clients.len() {
                let c = clients.get_mut(i).unwrap();
                let cli_evts = c.handle_updated_context(context)?;
                evts.extend_from_slice(&cli_evts);
            }

            if let Some(dispatch) = context.get_dispatch() {
                println!("* Context dispatch: {:?}", dispatch);
            }
        }
        Ok(())
    }

    pub fn get_state(&self) -> &H {
        &self.handler
    }

    pub fn get_mut_state(&mut self) -> &mut H {
        &mut self.handler
    }
}
