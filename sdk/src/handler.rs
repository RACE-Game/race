#![cfg(target_arch = "wasm32")]

use std::mem::swap;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::effect::Effect;
use race_core::encryptor::EncryptorT;
use race_core::engine::{general_handle_event, general_init_state, post_handle_event, InitAccount};
use race_core::error::{Error, Result};

use js_sys::WebAssembly::{Instance, Memory};
use js_sys::{Function, Object, Reflect, Uint8Array, WebAssembly};
use race_core::event::Event;
use race_core::types::GameBundle;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct Handler {
    pub instance: Instance,
    pub encryptor: Arc<dyn EncryptorT>,
}

impl Handler {
    pub async fn from_bundle(game_bundle: GameBundle, encryptor: Arc<dyn EncryptorT>) -> Self {
        let mut buffer = game_bundle.data;
        let mem_descriptor = Object::new();
        Reflect::set(&mem_descriptor, &"shared".into(), &true.into()).map_err(|_| {
            Error::WasmInitializationError("Failed to create mem descriptor".into())
        })?;
        Reflect::set(&mem_descriptor, &"maximum".into(), &100.into()).map_err(|_| {
            Error::WasmInitializationError("Failed to create mem descriptor".into())
        })?;
        Reflect::set(&mem_descriptor, &"initial".into(), &100.into()).map_err(|_| {
            Error::WasmInitializationError("Failed to create mem descriptor".into())
        })?;
        let memory = WebAssembly::Memory::new(&mem_descriptor).map_err(|_| {
            Error::WasmInitializationError("Failed to get WASM memory object".into())
        })?;
        let import_obj = Object::new();
        Reflect::set(&import_obj, &"memory".into(), &memory).map_err(|_| {
            Error::WasmInitializationError("Failed to set WASM memory object".into())
        })?;
        let a = JsFuture::from(WebAssembly::instantiate_buffer(&buffer, &import_obj))
            .await
            .map_err(|_| Error::WasmInitializationError("Failed to instantiate buffer".into()))?;
        let instance: Instance = Reflect::get(&a, &"instance".into())
            .map_err(|_| Error::WasmInitializationError("Failed to get WASM instance".into()))?
            .dyn_into()
            .map_err(|_| Error::WasmInitializationError("Failed to get WASM instance".into()))?;
        Ok(Self {
            instance,
            encryptor,
        })
    }

    fn custom_init_state(
        &self,
        context: &mut GameContext,
        init_account: &InitAccount,
    ) -> Result<()> {
        let exports = self.instance.exports();
        let mem = Reflect::get(exports.as_ref(), &"memory".into())
            .map_err(|_| Error::WasmExecutionError("Failed to get memory object".into()))?
            .dyn_into::<Memory>()
            .map_err(|_| Error::WasmExecutionError("Failed to get memory object".into()))?;
        mem.grow(4);
        let buf = Uint8Array::new(&mem.buffer());

        // serialize effect
        let mut effect = Effect::from_context(context);
        let effect_vec = effect
            .try_to_vec()
            .map_err(|_| Error::WasmExecutionError("Failed to serialize effect".into()))?;
        let effect_size = effect_vec.len();
        let effect_arr = Uint8Array::new_with_length(effect_size as _);
        effect_arr.copy_from(&effect_vec);

        // serialize init_account
        let init_account_vec = init_account
            .try_to_vec()
            .map_err(|_| Error::WasmExecutionError("Failed to serialize init_account".into()))?;
        let init_account_size = init_account_vec.len();
        let init_account_arr = Uint8Array::new_with_length(init_account_size as _);
        init_account_arr.copy_from(&init_account_vec);

        // copy effect and init_account into wasm memory
        let mut offset = 1u32;
        buf.set(&effect_arr, offset);
        offset += effect_size as u32;
        buf.set(&init_account_arr, offset);

        // call event handler
        let init_state = Reflect::get(exports.as_ref(), &"init_state".into())
            .map_err(|_| Error::WasmExecutionError("Failed to resolve init_state function".into()))?
            .dyn_into::<Function>()
            .map_err(|_| {
                Error::WasmExecutionError("Failed to resolve init_state function".into())
            })?;

        let new_effect_size = init_state
            .call2(
                &JsValue::undefined(),
                &effect_size.into(),
                &init_account_size.into(),
            )
            .map_err(|_| Error::WasmExecutionError("WASM invocation error".into()))?
            .as_f64()
            .ok_or(Error::WasmExecutionError(
                "WASM result parsing error".into(),
            ))?;

        let new_effect_vec = Uint8Array::new(&mem.buffer()).to_vec();
        let new_effect_slice = &new_effect_vec[1..(1 + new_effect_size as usize)];
        effect = Effect::try_from_slice(&new_effect_slice)
            .map_err(|_| Error::WasmExecutionError("Failed to deserialize effect".into()))?;

        if let Some(e) = effect.__take_error() {
            Err(e)
        } else {
            context.apply_effect(effect)
        }
    }

    fn custom_handle_event(&self, context: &mut GameContext, event: &Event) -> Result<()> {
        let exports = self.instance.exports();
        let mem = Reflect::get(exports.as_ref(), &"memory".into())
            .map_err(|_| Error::WasmExecutionError("Failed to get memory object".into()))?
            .dyn_into::<Memory>()
            .map_err(|_| Error::WasmExecutionError("Failed to get memory object".into()))?;
        let buf = Uint8Array::new(&mem.buffer());

        // serialize effect
        let mut effect = Effect::from_context(context);
        let effect_vec = effect
            .try_to_vec()
            .map_err(|_| Error::WasmExecutionError("Failed to serialize effect".into()))?;
        let effect_size = effect_vec.len();
        let effect_arr = Uint8Array::new_with_length(effect_size as _);
        effect_arr.copy_from(&effect_vec);

        // serialize event
        let event_vec = event
            .try_to_vec()
            .map_err(|_| Error::WasmExecutionError("Failed to serialize event".into()))?;
        let event_size = event_vec.len();
        let event_arr = Uint8Array::new_with_length(event_size as _);
        event_arr.copy_from(&event_vec);

        // copy context and event into wasm memory
        let mut offset = 1u32;
        buf.set(&effect_arr, offset);
        offset += effect_size as u32;
        buf.set(&event_arr, offset);

        // call event handler
        let handle_event = Reflect::get(exports.as_ref(), &"handle_event".into())
            .map_err(|_| {
                Error::WasmExecutionError("Failed to resolve handle_event function".into())
            })?
            .dyn_into::<Function>()
            .map_err(|_| {
                Error::WasmExecutionError("Failed to resolve handle_event function".into())
            })?;

        let new_effect_size = handle_event
            .call2(
                &JsValue::undefined(),
                &effect_size.into(),
                &event_size.into(),
            )
            .map_err(|_| Error::WasmExecutionError("WASM invocation error".into()))?
            .as_f64()
            .ok_or(Error::WasmExecutionError(
                "WASM result parsing error".into(),
            ))?;

        let new_effect_vec = Uint8Array::new(&mem.buffer()).to_vec();
        let new_effect_slice = &new_effect_vec[1..(1 + new_effect_size as usize)];
        effect = Effect::try_from_slice(&new_effect_slice)
            .map_err(|_| Error::WasmExecutionError("Failed to deserialize effect".into()))?;

        if let Some(e) = effect.__take_error() {
            Err(e)
        } else {
            context.apply_effect(effect)
        }
    }

    pub fn handle_event(&self, context: &mut GameContext, event: &Event) -> Result<()> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, event, self.encryptor.as_ref())?;
        self.custom_handle_event(&mut new_context, event)?;
        post_handle_event(context, &mut new_context)?;
        // TODO, the settlement is ignored
        // We should start a independent task to verify the settlements on-chain
        // Here we should send a verification job to the task
        new_context.apply_and_take_settles()?;
        // info!(format!("context: {:?}", new_context));
        swap(context, &mut new_context);
        Ok(())
    }

    pub fn init_state(&self, context: &mut GameContext, init_account: &InitAccount) -> Result<()> {
        let mut new_context = context.clone();
        general_init_state(&mut new_context, init_account)?;
        self.custom_init_state(&mut new_context, init_account)?;
        swap(context, &mut new_context);
        Ok(())
    }
}
