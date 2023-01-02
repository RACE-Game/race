use std::mem::swap;
use std::path::PathBuf;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::engine::{after_handle_event, general_handle_event, general_init_state};
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::GameAccount;
use wasmer::{imports, Instance, Module, Store, TypedFunction};

pub struct WrappedHandler {
    store: Store,
    instance: Instance,
}

impl WrappedHandler {
    /// Load WASM bundle by game address
    pub async fn load_by_addr(addr: &str, transport: &dyn TransportT) -> Result<Self> {
        let mut store = Store::default();
        let game_bundle = transport.get_game_bundle(addr).await.ok_or(Error::GameBundleNotFound)?;
        let module = Module::from_binary(&store, &game_bundle.data).or(Err(Error::MalformedGameBundle))?;
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self { store, instance })
    }

    /// Load WASM bundle by relative path
    /// This function is used for testing.
    #[allow(dead_code)]
    pub fn load_by_path(path: PathBuf) -> Result<Self> {
        let mut store = Store::default();
        let module = Module::from_file(&store, path).expect("Fail to load module");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self { store, instance })
    }

    pub fn custom_init_state(&mut self, context: &mut GameContext, init_account: &GameAccount) -> Result<()> {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
        memory.grow(&mut self.store, 1).expect("Failed to grow");
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
            .call(&mut self.store, context_bs.len() as _, init_account_bs.len() as _)
            .expect("Handle event error");
        println!("Len: {:?}", len);
        let mut buf = vec![0; len as _];
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
        Ok(())
    }

    fn custom_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
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
        mem_view.write(offset as _, &event_bs).expect("Failed to write event");
        let len = handle_event
            .call(&mut self.store, context_bs.len() as _, event_bs.len() as _)
            .expect("Handle event error");
        println!("Len: {:?}", len);
        let mut buf = vec![0; len as _];
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
        Ok(())
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, event)?;
        self.custom_handle_event(&mut new_context, event)?;
        after_handle_event(&mut new_context)?;
        swap(context, &mut new_context);
        Ok(())
    }

    pub fn init_state(&mut self, context: &mut GameContext, init_account: &GameAccount) -> Result<()> {
        let mut new_context = context.clone();
        general_init_state(&mut new_context, init_account)?;
        self.custom_init_state(&mut new_context, init_account)?;
        swap(context, &mut new_context);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::tests::game_account_with_account_data;
    use race_core::types::GameAccount;

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        counter_value_default: u64,
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData {
            counter_value_default: 42,
        };
        game_account_with_account_data(data)
    }

    fn make_wrapped_handler() -> WrappedHandler {
        WrappedHandler::load_by_path("../target/wasm32-unknown-unknown/release/race_example_minimal.wasm".into())
            .unwrap()
    }

    #[test]
    fn test_init_state() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        hdlr.init_state(&mut ctx, &game_account).unwrap();
        assert_eq!(
            "{\"counter_value\":42,\"counter_players\":0}",
            ctx.get_handler_state_json()
        );
    }

    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        let event = Event::Join {
            player_addr: "FAKE_ADDR".into(),
            balance: 1000,
        };
        hdlr.init_state(&mut ctx, &game_account).unwrap();
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!(
            "{\"counter_value\":42,\"counter_players\":1}",
            ctx.get_handler_state_json()
        );
    }
}
