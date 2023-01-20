#![cfg(target_arch = "wasm32")]

use std::mem::swap;

use borsh::{BorshSerialize, BorshDeserialize};
use race_core::context::GameContext;
use race_core::engine::general_handle_event;
use race_core::error::Result;

use js_sys::WebAssembly::{Instance, Memory};
use js_sys::{Function, Object, Reflect, Uint8Array, WebAssembly};
use race_core::event::Event;
use race_core::types::GameBundle;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct Handler {
    pub instance: Instance,
}

impl Handler {
    pub async fn from_bundle(mut game_bundle: GameBundle) -> Self {
        let bin = game_bundle.data.as_mut_slice();
        let mem_descriptor = Object::new();
        Reflect::set(&mem_descriptor, &"shared".into(), &true.into()).unwrap();
        Reflect::set(&mem_descriptor, &"maximum".into(), &100.into()).unwrap();
        Reflect::set(&mem_descriptor, &"initial".into(), &100.into()).unwrap();
        let memory = WebAssembly::Memory::new(&mem_descriptor).unwrap();
        let import_obj = Object::new();
        Reflect::set(&import_obj, &"memory".into(), &memory).unwrap();
        let a = JsFuture::from(WebAssembly::instantiate_buffer(bin, &import_obj))
            .await
            .unwrap();
        let instance: Instance = Reflect::get(&a, &"instance".into())
            .unwrap()
            .dyn_into()
            .unwrap();
        Self { instance }
    }

    fn custom_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let exports = self.instance.exports();
        let mem = Reflect::get(exports.as_ref(), &"memory".into())
            .unwrap()
            .dyn_into::<Memory>()
            .expect("Can't get memory");
        let buf = Uint8Array::new(&mem.buffer());

        // serialize context
        let context_vec = context.try_to_vec().unwrap();
        let context_size = context_vec.len();
        let context_arr = Uint8Array::new_with_length(context_size as _);
        context_arr.copy_from(&context_vec);

        // serialize event
        let event_vec = event.try_to_vec().unwrap();
        let event_size = event_vec.len();
        let event_arr = Uint8Array::new_with_length(event_size as _);
        event_arr.copy_from(&event_vec);

        // copy context and event into wasm memory
        let mut offset = 1u32;
        buf.set(&context_arr, offset);
        offset += context_size as u32;
        buf.set(&event_arr, offset);

        // call event handler
        let handle_event = Reflect::get(exports.as_ref(), &"handle_event".into())
            .unwrap()
            .dyn_into::<Function>()
            .expect("Can't get handle_event");
        let new_context_size = handle_event
            .call2(
                &JsValue::undefined(),
                &context_size.into(),
                &event_size.into(),
            )
            .unwrap()
            .as_f64()
            .unwrap() as usize;

        let new_context_vec = Uint8Array::new(&mem.buffer()).to_vec();
        let new_context_slice = &new_context_vec[1..(1 + new_context_size)];
        *context = GameContext::try_from_slice(&new_context_slice).unwrap();

        Ok(())
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, event)?;
        self.custom_handle_event(&mut new_context, event)?;
        swap(context, &mut new_context);
        Ok(())
    }
}
