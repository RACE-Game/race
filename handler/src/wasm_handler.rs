use std::path::PathBuf;

use borsh::BorshDeserialize;
use race_api::effect::Effect;
use race_api::event::Event;
use race_api::init_account::InitAccount;
use race_core::error::{Error, Result};
use race_core::types::GameBundle;
use tracing::{info, error};
use wasmer::{imports, Instance, Module, Store, TypedFunction};

use crate::handler::HandlerT;

fn log_execution_context(effect_bs: &Vec<u8>, event_bs: &Vec<u8>) {
    info!("Execution context");
    info!("===== Effect Bytes =====");
    info!("{:?}", effect_bs);
    info!("===== Event Bytes =====");
    info!("{:?}", event_bs);
    info!("=================");
}

/// Create a new empty Effect for initialization.
fn init_effect() -> Effect {
    Effect {
        is_init: true,
        curr_sub_game_id: 1,
        timestamp: 0,
        nodes_count: 1,
        ..Default::default()
    }
}

pub struct WasmHandler {
    store: Store,
    instance: Instance,
}

impl HandlerT for WasmHandler {
    fn handle_event(
        &mut self,
        effect: &Effect,
        event: &Event,
    ) -> Result<Effect> {
        let effect = self.custom_handle_event(effect, event)?;
        Ok(effect)
    }

    // Initialize game state
    fn init_state(
        &mut self,
        init_account: &InitAccount,
    ) -> Result<Effect> {
        let effect = self.custom_init_state(init_account)?;
        Ok(effect)
    }
}

impl WasmHandler {
    /// Load WASM bundle
    pub async fn load_by_bundle(
        bundle: &GameBundle,
    ) -> Result<Self> {
        let mut store = Store::default();
        let module =
            Module::from_binary(&store, &bundle.data).or(Err(Error::MalformedGameBundle))?;
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self {
            store,
            instance,
        })
    }

    /// Load WASM bundle by relative path
    /// This function is used for testing.
    #[allow(dead_code)]
    pub fn load_by_path(path: PathBuf) -> Result<Self> {
        let mut store = Store::default();
        let module = Module::from_file(&store, path).expect("Fail to load module");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self {
            store,
            instance,
        })
    }

    pub fn custom_init_state(
        &mut self,
        init_account: &InitAccount,
    ) -> Result<Effect> {

        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;

        memory
            .grow(&mut self.store, 4)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let init_state: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "init_state")
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let mem_view = memory.view(&self.store);
        let effect = init_effect();
        let effect_bs =
            borsh::to_vec(&effect).map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let init_account_bs = borsh::to_vec(&init_account)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let mut offset = 1u32;
        mem_view
            .write(offset as _, &effect_bs)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        offset = offset
            .checked_add(effect_bs.len() as _)
            .ok_or(Error::WasmMemoryOverflow)?;
        mem_view
            .write(offset as _, &init_account_bs)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let len = init_state
            .call(
                &mut self.store,
                effect_bs.len() as _,
                init_account_bs.len() as _,
            )
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;

        match len {
            0 => {
                return Err(Error::WasmInitializationError(
                    "Serializing effect failed".into(),
                ))
            }
            1 => {
                return Err(Error::WasmInitializationError(
                    "Deserializing effect failed".into(),
                ))
            }
            2 => {
                return Err(Error::WasmInitializationError(
                    "Deserializing event failed".into(),
                ))
            }
            _ => (),
        }

        let mut buf = vec![0; len as _];
        let mem_view = memory.view(&self.store);
        mem_view
            .read(1u64, &mut buf)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;
        let effect = Effect::try_from_slice(&buf)
            .map_err(|e| Error::WasmInitializationError(e.to_string()))?;

        Ok(effect)
    }

    fn custom_handle_event(
        &mut self,
        effect: &Effect,
        event: &Event,
    ) -> Result<Effect> {
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
        let effect_bs =
            borsh::to_vec(effect).map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let event_bs =
            borsh::to_vec(&event).map_err(|e| Error::WasmExecutionError(e.to_string()))?;
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
            .map_err(|e| {
                log_execution_context(&effect_bs, &event_bs);
                Error::WasmExecutionError(e.to_string())
            })?;

        match len {
            0 => {
                error!("Effect: {:?}", effect_bs);
                return Err(Error::WasmExecutionError(
                    "Serializing effect failed".into(),
                ));
            }
            1 => {
                error!("Effect: {:?}", effect_bs);
                return Err(Error::WasmExecutionError(
                    "Deserializing effect failed".into(),
                ));
            }
            2 => {
                error!("Effect: {:?}", effect_bs);
                return Err(Error::WasmExecutionError(
                    "Deserializing event failed".into(),
                ));
            }
            _ => (),
        }

        let mut buf = vec![0; len as _];
        let mem_view = memory.view(&self.store);
        mem_view
            .read(1u64, &mut buf)
            .map_err(|e| Error::WasmExecutionError(e.to_string()))?;
        let effect =
            Effect::try_from_slice(&buf).map_err(|e| Error::WasmExecutionError(e.to_string()))?;

        Ok(effect)
    }
}

#[cfg(test)]
mod tests {
    use borsh::BorshSerialize;
    use race_api::{
        prelude::{CustomEvent, HandleError},
        types::GameStatus,
    };
    use race_test::prelude::*;

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

    fn setup_game() -> (GameAccount, TestClient) {
        let data = MinimalAccountData { init_n: 42 };
        let mut transactor = TestClient::transactor("transactor");
        let acc = TestGameAccountBuilder::default()
            .set_transactor(&mut transactor)
            .with_data(data)
            .build();
        (acc, transactor)
    }

    fn make_wrapped_handler() -> WasmHandler {
        let proj_root = project_root::get_project_root().expect("No project root found");
        let bundle_path = proj_root.join("examples/minimal/minimal.wasm");
        WasmHandler::load_by_path(bundle_path).unwrap()
    }

    #[ignore]
    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let (game_account, _tx) = setup_game();
        let mut ctx = GameContext::try_new(&game_account, None).unwrap();
        let event = Event::GameStart;
        let init_account = ctx.init_account();
        hdlr.init_state(&mut ctx, &init_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!(
            &vec![42u8, 0, 0, 0, 0, 0, 0, 0],
            ctx.get_handler_state_raw()
        );
        assert_eq!(ctx.get_status(), GameStatus::Running);
    }

    #[ignore]
    #[test]
    fn test_handle_custom_event() {
        let mut hdlr = make_wrapped_handler();
        let (game_account, _tx) = setup_game();
        let mut ctx = GameContext::try_new(&game_account, None).unwrap();
        let event = Event::custom(0, &MinimalEvent::Increment(1));
        let init_account = ctx.init_account();
        hdlr.init_state(&mut ctx, &init_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!(
            &vec![43u8, 0, 0, 0, 0, 0, 0, 0],
            ctx.get_handler_state_raw()
        );
    }
}
