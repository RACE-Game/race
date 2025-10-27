use crate::session::Session;
use race_components::CloseReason;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use serde::Serialize;

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>
}

impl SessionManager {
    pub fn new() -> Self {
        let sessions = Arc::new(RwLock::new(HashMap::default()));
        Self { sessions }
    }

    pub async fn start_new_replay_session(
        &self,
        game_addr: String,
        settle_version: u64, // Specify where to start
    ) -> Option<JoinHandle<CloseReason>> {

        None
    }

    pub async fn stop_replay_session() {

    }
}
