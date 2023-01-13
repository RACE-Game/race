use std::mem::swap;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::GameContext,
    engine::{after_handle_event, general_handle_event},
    error::Result,
    event::Event,
    types::GameBundle,
};
use wasmer::{imports, Instance, Module, Store, TypedFunction};

pub struct Handler {
    pub store: Store,
    pub instance: Instance,
}

impl Handler {
    pub fn new(game_bundle: GameBundle) -> Self {
        let mut store = Store::default();
        let module = Module::from_binary(&store, &game_bundle.data).expect("Failed to load bundle");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Self { store, instance }
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
}
