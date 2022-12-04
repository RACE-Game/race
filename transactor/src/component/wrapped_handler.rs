use std::path::PathBuf;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::GameAccount;
use wasmer::{imports, Instance, Memory, MemoryType, Module, Store, TypedFunction};

pub struct WrappedHandler {
    store: Store,
    instance: Instance,
}

pub struct WrappedHandlerState {}

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

    pub fn init_state(&mut self, context: &mut GameContext, init_account: &GameAccount) {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
        memory.grow(&mut self.store, 10).expect("Failed to grow");
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
        let mut buf = Vec::with_capacity(len as _);
        buf.resize(len as _, 0);
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
        memory.grow(&mut self.store, 10).expect("Failed to grow");
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
        let mut buf = Vec::with_capacity(len as _);
        buf.resize(len as _, 0);
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use race_core::{context::DispatchEvent, types::GameAccount};

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        counter: u64,
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData { counter: 42 }.try_to_vec().unwrap();
        GameAccount {
            addr: "ACC ADDR".into(),
            game_addr: "GAME ADDR".into(),
            settle_serial: 0,
            access_serial: 0,
            players: vec![],
            data_len: data.len() as _,
            data,
        }
    }

    fn make_wrapped_handler() -> WrappedHandler {
        WrappedHandler::load_by_path("../target/wasm32-unknown-unknown/release/minimal.wasm".into()).unwrap()
    }

    #[test]
    fn test_init_state() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        hdlr.init_state(&mut ctx, &game_account);
        assert_eq!("{\"counter\":42}", ctx.state_json);
    }

    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        let event = Event::Ready {
            player_addr: "FAKE_ADDR".into(),
            timestamp: 0,
        };
        hdlr.init_state(&mut ctx, &game_account);
        hdlr.handle_event(&mut ctx, &event);
        assert_eq!(
            Some(DispatchEvent::new(Event::Custom("{\"Increase\":1}".into()), 0)),
            ctx.dispatch
        );
        assert_eq!("{\"counter\":42}", ctx.state_json);
    }
}
