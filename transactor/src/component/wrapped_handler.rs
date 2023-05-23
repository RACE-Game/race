use std::mem::swap;
use std::path::PathBuf;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::effect::Effect;
use race_core::encryptor::EncryptorT;
use race_core::engine::{general_handle_event, general_init_state, post_handle_event, InitAccount};
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::types::{GameBundle, Settle};
use race_encryptor::Encryptor;
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
        let mut store = Store::default();
        let module =
            Module::from_binary(&store, &bundle.data).or(Err(Error::MalformedGameBundle))?;
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
        init_account: &InitAccount,
    ) -> Result<()> {
        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;

        memory
            .grow(&mut self.store, 4)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let init_state: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "init_state")
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mem_view = memory.view(&self.store);
        let effect = Effect::from_context(context);
        let effect_bs = effect
            .try_to_vec()
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let init_account_bs = init_account
            .try_to_vec()
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mut offset = 1u32;
        mem_view
            .write(offset as _, &effect_bs)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        offset = offset
            .checked_add(effect_bs.len() as _)
            .ok_or(Error::WasmMemoryOverflow)?;
        mem_view
            .write(offset as _, &init_account_bs)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let len = init_state
            .call(
                &mut self.store,
                effect_bs.len() as _,
                init_account_bs.len() as _,
            )
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;

        if len == 0 {
            return Err(Error::WasmExecutionError("Internal error".into()));
        }

        let mut buf = vec![0; len as _];
        let mem_view = memory.view(&self.store);
        mem_view
            .read(1u64, &mut buf)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mut effect =
            Effect::try_from_slice(&buf).map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        if let Some(e) = effect.__take_error() {
            Err(e.into())
        } else {
            context.apply_effect(effect)
        }
    }

    fn custom_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let handle_event: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "handle_event")
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mem_view = memory.view(&self.store);
        let effect = Effect::from_context(context);
        let effect_bs = effect
            .try_to_vec()
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let event_bs = event
            .try_to_vec()
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mut offset = 1u32;
        mem_view
            .write(offset as _, &effect_bs)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        offset = offset
            .checked_add(effect_bs.len() as _)
            .ok_or(Error::WasmMemoryOverflow)?;
        mem_view
            .write(offset as _, &event_bs)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let len = handle_event
            .call(&mut self.store, effect_bs.len() as _, event_bs.len() as _)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;

        if len == 0 {
            return Err(Error::WasmExecutionError("Internal error".into()));
        }

        let mut buf = vec![0; len as _];
        let mem_view = memory.view(&self.store);
        mem_view
            .read(1u64, &mut buf)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let mut effect =
            Effect::try_from_slice(&buf).map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        if let Some(e) = effect.__take_error() {
            Err(e.into())
        } else {
            context.apply_effect(effect)
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
        init_account: &InitAccount,
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
    use race_core::{
        context::GameStatus,
        prelude::{CustomEvent, HandleError},
        types::GameAccount,
    };
    use race_test::*;

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        init_n: u64,
    }

    #[derive(BorshDeserialize, BorshSerialize)]
    enum MinimalEvent {
        Increment(u64),
    }

    impl CustomEvent for MinimalEvent {
        fn try_parse(slice: &[u8]) -> std::result::Result<Self, HandleError> {
            Ok(MinimalEvent::try_from_slice(slice).or(Err(HandleError::MalformedCustomEvent))?)
        }
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData { init_n: 42 };
        TestGameAccountBuilder::default()
            .with_data(data)
            .add_servers(1)
            .build()
    }

    fn make_wrapped_handler() -> WrappedHandler {
        let proj_root = project_root::get_project_root().expect("No project root found");
        let bundle_path = proj_root.join("target/race_example_minimal.wasm");
        WrappedHandler::load_by_path(bundle_path).unwrap()
    }

    #[test]
    fn test_init_state() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let init_account = InitAccount::from_game_account(&game_account);
        let mut ctx = GameContext::try_new(&game_account).unwrap();
        hdlr.init_state(&mut ctx, &init_account).unwrap();
        assert_eq!(&vec![42u8, 0, 0, 0, 0, 0, 0, 0], ctx.get_handler_state_raw());
    }

    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::try_new(&game_account).unwrap();
        let event = Event::GameStart {
            access_version: game_account.access_version,
        };
        let init_account = InitAccount::from_game_account(&game_account);
        hdlr.init_state(&mut ctx, &init_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!(&vec![42u8, 0, 0, 0, 0, 0, 0, 0], ctx.get_handler_state_raw());
        assert_eq!(ctx.get_status(), GameStatus::Running);
    }

    #[test]
    fn test_handle_custom_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::try_new(&game_account).unwrap();
        let event = Event::custom("Alice", &MinimalEvent::Increment(1));
        let init_account = InitAccount::from_game_account(&game_account);
        hdlr.init_state(&mut ctx, &init_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!(&vec![43u8, 0, 0, 0, 0, 0, 0, 0], ctx.get_handler_state_raw());
    }
}
