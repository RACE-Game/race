//! We need a handler for event handling.

use std::sync::Arc;
use race_core::types::accounts::GameBundle;
use race_encryptor::Encryptor;
use race_transactor_components::{
    event_loop::EventLoop,
    event_bus::EventBus,
    wrapped_handler::WrappedHandler,
};

struct EventHandler {

}


impl EventHandler {
    pub fn new(game_bundle: &GameBundle, records: Vec<Record>) -> Result<Self, ReplayerError> {
        let encryptor = Arc::new(Encryptor::default());
        let handler = WrappedHandler::load_by_bundle(game_bundle, encryptor)?;

    }
}
