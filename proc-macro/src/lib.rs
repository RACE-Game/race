extern crate proc_macro;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn game_handler(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let s = parse_macro_input!(input as ItemStruct);
    let s_idt = s.clone().ident;

    TokenStream::from(quote! {

        #s

        pub fn read_ptr<T: BorshDeserialize>(ptr: &mut *mut u8, size: u32) -> T {
            let slice = unsafe { core::slice::from_raw_parts_mut(*ptr, size as _) };
            let parsed = T::try_from_slice(&slice).expect("Borsh deserialize error");
            *ptr = unsafe { ptr.add(size as _) };
            parsed
        }

        pub fn write_ptr<T: BorshSerialize>(ptr: &mut *mut u8, data: T) -> u32 {
            let vec = data.try_to_vec().expect("Borsh serialize error");
            unsafe { std::ptr::copy(vec.as_ptr(), *ptr, vec.len()) }
            *ptr = unsafe { ptr.add(vec.len() as _) };
            vec.len() as _
        }

        #[no_mangle]
        pub extern "C" fn handle_event(context_size: u32, event_size: u32) -> u32 {
            let mut ptr = 1 as *mut u8;
            let mut context: race_core::context::GameContext = read_ptr(&mut ptr, context_size);
            let event: race_core::event::Event = read_ptr(&mut ptr, event_size);
            let mut handler: #s_idt = serde_json::from_str(&context.state_json).unwrap();
            handler.handle_event(&mut context, event).unwrap();
            context.state_json = serde_json::to_string(&handler).unwrap();
            let mut ptr = 1 as *mut u8;
            write_ptr(&mut ptr, context)
        }

        #[no_mangle]
        pub extern "C" fn init_state(context_size: u32, init_account_size: u32) -> u32 {
            let mut ptr = 1 as *mut u8;
            let mut context: race_core::context::GameContext = read_ptr(&mut ptr, context_size);
            let init_account: race_core::types::GameAccount = read_ptr(&mut ptr, init_account_size);
            let handler = #s_idt::init_state(&mut context, init_account).unwrap();
            context.state_json = serde_json::to_string(&handler).unwrap();
            let mut ptr = 1 as *mut u8;
            write_ptr(&mut ptr, context)
        }
    })
}
