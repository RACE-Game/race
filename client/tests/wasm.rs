#![cfg(target_arch = "wasm32")]
#![cfg(test)]

extern crate race_client;
extern crate wasm_bindgen_test;

use race_client::WrappedHandler;
use race_core::{context::{GameContext, DispatchEvent}, event::Event};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_hello() {
    assert_eq!(2, 1 + 1);
}

#[wasm_bindgen_test]
async fn test_handle_event() {
    let mut hdlr = WrappedHandler::load_by_addr("facade-program-addr").await.unwrap();
    let mut context = GameContext::default();
    let event = Event::Join {
        player_addr: "FAKE PLAYER ADDR".into(),
        timestamp: 0,
    };
    hdlr.handle_event(&mut context, event);
    assert_eq!(
        Some(DispatchEvent::new(Event::Custom("{\"Increase\":1}".into()), 0)),
        context.dispatch
    );
}
