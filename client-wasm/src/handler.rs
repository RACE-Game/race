#![cfg(target_arch="wasm32")]
use std::mem::swap;

use borsh::{BorshDeserialize, BorshSerialize};
use js_sys::WebAssembly::{Instance, Memory};
use js_sys::{Function, Object, Reflect, Uint8Array, WebAssembly, JSON};

use race_core::{
    context::GameContext,
    engine::{after_handle_event, general_handle_event, general_init_state},
    error::Result,
    event::Event,
    types::{GameAccount, GameBundle},
};

pub struct Handler {
    pub store: Store,
    pub instance: Instance,
}

impl Handler {
    pub fn new(game_bundle: GameBundle) -> Self {

//         let wasm = bundle.data.as_mut_slice();
//         let mem_descriptor = Object::new();
//         Reflect::set(&mem_descriptor, &"shared".into(), &true.into()).unwrap();
//         // 64k * 1000, TODO, use fewer memory
//         Reflect::set(&mem_descriptor, &"maximum".into(), &1000.into()).unwrap();
//         Reflect::set(&mem_descriptor, &"initial".into(), &1000.into()).unwrap();
//         console_log!("Initializing linear memory...");
//         let memory = WebAssembly::Memory::new(&mem_descriptor).unwrap();
//         console_log!("Linear memory created");
//         let import_obj = Object::new();
//         Reflect::set(&import_obj, &"memory".into(), &memory).unwrap();
//         let a = JsFuture::from(WebAssembly::instantiate_buffer(wasm, &import_obj)).await?;
//         let instance: WebAssembly::Instance = Reflect::get(&a, &"instance".into())?.dyn_into()?;
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
