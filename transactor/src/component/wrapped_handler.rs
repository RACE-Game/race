use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::event::Event;
use wasmer::{imports, Instance, Memory, MemoryType, Module, Store, TypedFunction};

pub struct WrappedHandler {
    store: Store,
    instance: Instance,
}

pub struct WrappedHandlerState {}

impl WrappedHandler {
    pub fn new() -> Result<Self> {
        let mut store = Store::default();
        let module = Module::from_file(&store, "../target/wasm32-unknown-unknown/release/minimal.wasm")
            .expect("Fail to load module");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self { store, instance })
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: Event) {
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
    use race_core::context::DispatchEvent;

    use super::*;

    #[test]
    fn test_handle_event() {
        let mut hdlr = WrappedHandler::new().unwrap();
        let mut ctx = GameContext::default();
        let event = Event::Ready {
            player_addr: "FAKE_ADDR".into(),
            timestamp: 0,
        };
        hdlr.handle_event(&mut ctx, event);
        assert_eq!(
            Some(DispatchEvent::new(Event::Custom("{\"Increase\":1}".into()), 0)),
            ctx.dispatch
        );
        assert_eq!(
            "{\"counter\":0}",
            ctx.state_json.unwrap()
        );
    }
}
