use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum RequestFrame {
    SendEvent,
    GetState,
}
