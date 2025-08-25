extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

/// A macro to generate boilerplate code for using in wasm.
///
/// ```
/// use race_api::prelude::*;
/// use race_proc_macro::game_handler;
///
/// #[derive(BorshDeserialize, BorshSerialize)]
/// #[game_handler]
/// struct S {}
///
/// impl GameHandler for S {
///
///     fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self> {
///         Ok(Self {})
///     }
///
///     fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
///         Ok(())
///     }
///
///     fn balances(&self) -> Vec<PlayerBalance> {
///         vec![]
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn game_handler(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as ItemStruct);
    let s_idt = s.clone().ident;

    TokenStream::from(quote! {

        #s

        pub fn read_ptr<T: BorshDeserialize>(ptr: &mut *mut u8, size: u32) -> Option<T> {
            let slice = unsafe { core::slice::from_raw_parts_mut(*ptr, size as _) };
            if let Ok(parsed) = T::try_from_slice(&slice) {
                *ptr = unsafe { ptr.add(size as _) };
                Some(parsed)
            } else {
                None
            }
        }

        pub fn write_ptr<T: BorshSerialize>(ptr: &mut *mut u8, data: T) -> u32 {
            if let Ok(vec) = borsh::to_vec(&data) {
                unsafe { std::ptr::copy(vec.as_ptr(), *ptr, vec.len()) }
                vec.len() as _
            } else {
                0
            }
        }

        #[no_mangle]
        pub extern "C" fn handle_event(effect_size: u32, event_size: u32) -> u32 {
            let mut ptr = 1 as *mut u8;
            let mut effect: race_api::effect::Effect = if let Some(effect) =  read_ptr(&mut ptr, effect_size) {
                effect
            } else {
                return 1
            };
            let event: race_api::event::Event = if let Some(event) = read_ptr(&mut ptr, event_size) {
                event
            } else {
                return 2
            };

            let mut handler: #s_idt = effect.__handler_state();
            match handler.handle_event(&mut effect, event) {
                Ok(_) => effect.__set_handler_result(handler),
                Err(e) => effect.__set_error(e),
            }

            let mut ptr = 1 as *mut u8;
            write_ptr(&mut ptr, effect)
        }

        #[no_mangle]
        pub extern "C" fn init_state(effect_size: u32, init_account_size: u32) -> u32 {
            let mut ptr = 1 as *mut u8;
            let mut effect: race_api::effect::Effect = if let Some(effect) =  read_ptr(&mut ptr, effect_size) {
                effect
            } else {
                return 1
            };
            let init_account: race_api::init_account::InitAccount = if let Some(init_account) = read_ptr(&mut ptr, init_account_size) {
                init_account
            } else {
                return 2
            };
            match #s_idt::init_state(&mut effect, init_account) {
                Ok(handler) => effect.__set_handler_result(handler),
                Err(e) => effect.__set_error(e),
            }
            let mut ptr = 1 as *mut u8;
            write_ptr(&mut ptr, effect)
        }
    })
}
