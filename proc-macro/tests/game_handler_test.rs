use race_core::engine::GameHandler;
use race_proc_macro::game_handler;

#[game_handler]
struct MyGameHandler {}

impl GameHandler for MyGameHandler {
    fn init_state(context: &mut race_core::context::GameContext, init_account: race_core::types::GameAccount) -> race_core::error::Result<Self> {
        todo!()
    }

    fn handle_event(&mut self, context: &mut race_core::context::GameContext, event: race_core::event::Event) -> race_core::error::Result<()> {
        todo!()
    }
}

#[test]
fn test_macro() {
}
