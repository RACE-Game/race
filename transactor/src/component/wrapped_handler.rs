use std::mem::swap;
use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::encryptor::EncryptorT;
use race_core::engine::{general_handle_event, general_init_state, post_handle_event};
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::types::{GameAccount, GameBundle, Settle};
use race_encryptor::Encryptor;
use tracing::error;
use wasmer::{imports, Instance, Module, Store, TypedFunction};

pub struct WrappedHandler {
    store: Store,
    instance: Instance,
    encryptor: Arc<dyn EncryptorT>,
}

pub struct Effects {
    pub settles: Option<Vec<Settle>>,
}

impl WrappedHandler {
    /// Load WASM bundle by game address
    pub async fn load_by_bundle(
        bundle: &GameBundle,
        encryptor: Arc<dyn EncryptorT>,
    ) -> Result<Self> {
        let base64 = base64::prelude::BASE64_STANDARD;
        let mut buffer = Vec::with_capacity(1024);
        base64
            .decode_vec(&bundle.data, &mut buffer)
            .or(Err(Error::MalformedGameBundle))?;
        let mut store = Store::default();
        let module = Module::from_binary(&store, &buffer).or(Err(Error::MalformedGameBundle))?;
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self {
            store,
            instance,
            encryptor,
        })
    }

    /// Load WASM bundle by relative path
    /// This function is used for testing.
    #[allow(dead_code)]
    pub fn load_by_path(path: PathBuf) -> Result<Self> {
        let mut store = Store::default();
        let encryptor = Arc::new(Encryptor::default());
        let module = Module::from_file(&store, path).expect("Fail to load module");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self {
            store,
            instance,
            encryptor,
        })
    }

    pub fn custom_init_state(
        &mut self,
        context: &mut GameContext,
        init_account: &GameAccount,
    ) -> Result<()> {
        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .expect("Get memory failed");
        memory.grow(&mut self.store, 4).expect("Failed to grow");
        let init_state: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "init_state")
            .expect("Failed to get function");
        let mem_view = memory.view(&self.store);
        let context_bs = context.try_to_vec().unwrap();
        let init_account_bs = init_account.try_to_vec().unwrap();
        let mut offset = 1u64;
        mem_view
            .write(offset as _, &context_bs)
            .expect("Failed to write context");
        offset += context_bs.len() as u64;
        mem_view
            .write(offset as _, &init_account_bs)
            .expect("Failed to write init account");
        let len = init_state
            .call(
                &mut self.store,
                context_bs.len() as _,
                init_account_bs.len() as _,
            )
            .expect("Handle event error");
        let mut buf = vec![0; len as _];
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
        if let Some(e) = context.get_error() {
            Err(e.clone())
        } else {
            Ok(())
        }
    }

    fn custom_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .expect("Get memory failed");
        let handle_event: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "handle_event")
            .expect("Failed to get function");
        let mem_view = memory.view(&self.store);
        let context_bs = context.try_to_vec().unwrap();
        let event_bs = event.try_to_vec().unwrap();
        let mut offset = 1u64;
        mem_view
            .write(offset as _, &context_bs)
            .expect("Failed to write context");
        offset += context_bs.len() as u64;
        mem_view
            .write(offset as _, &event_bs)
            .expect("Failed to write event");
        let len = handle_event
            .call(&mut self.store, context_bs.len() as _, event_bs.len() as _)
            .map_err(|e| {
                error!("An error occurred in game handler: {:?}", e);
                e
            })
            .expect("Handle event error");
        let mut buf = vec![0; len as _];
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
        if let Some(e) = context.get_error() {
            Err(e.clone())
        } else {
            Ok(())
        }
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<Effects> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, event, self.encryptor.as_ref())?;
        self.custom_handle_event(&mut new_context, event)?;
        post_handle_event(context, &mut new_context)?;
        let settles = new_context.apply_and_take_settles()?;
        swap(context, &mut new_context);
        Ok(Effects { settles })
    }

    pub fn init_state(
        &mut self,
        context: &mut GameContext,
        init_account: &GameAccount,
    ) -> Result<()> {
        let mut new_context = context.clone();
        general_init_state(&mut new_context, init_account)?;
        self.custom_init_state(&mut new_context, init_account)?;
        swap(context, &mut new_context);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use race_core::types::{GameAccount, PlayerJoin};
    use race_test::*;

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        counter_value_default: u64,
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData {
            counter_value_default: 42,
        };
        TestGameAccountBuilder::default()
            .with_data(data)
            .add_servers(1)
            .build()
    }

    fn make_wrapped_handler() -> WrappedHandler {
        WrappedHandler::load_by_path(
            "../target/wasm32-unknown-unknown/release/race_example_counter.wasm".into(),
        )
        .unwrap()
    }

    #[test]
    fn test_init_state() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::try_new(&game_account).unwrap();
        hdlr.init_state(&mut ctx, &game_account).unwrap();
        assert_ne!("", ctx.get_handler_state_json());
    }

    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::try_new(&game_account).unwrap();
        let event = Event::Sync {
            new_players: vec![PlayerJoin {
                addr: "FAKE_ADDR".into(),
                balance: 1000,
                position: 0,
                access_version: ctx.get_access_version() + 1,
            }],
            new_servers: vec![],
            transactor_addr: transactor_account_addr(),
            access_version: ctx.get_access_version() + 1,
        };
        hdlr.init_state(&mut ctx, &game_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_ne!("", ctx.get_handler_state_json());
    }
}
