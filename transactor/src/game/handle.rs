
use tokio::sync::mpsc;
use crate::error::TransactorError;
use crate::component

pub enum Message {
    SendEvent,
    GetState,
}

pub struct Handle {
    pub addr: String,
    pub input: mpsc::Sender<MessageFrame>,
}

impl Handle {
    pub fn new(game_addr: &'str) {

    }
}
