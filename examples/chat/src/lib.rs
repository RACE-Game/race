use borsh::{BorshDeserialize, BorshSerialize};
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::types::GameAccount;
use race_core::{context::GameContext, engine::GameHandler};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum GameEvent {
    PublicMessage { text: String },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[game_handler]
struct Chat {
    messages: Vec<String>,
    num_of_clients: usize,
    num_of_servers: usize,
}

impl GameHandler for Chat {
    /// Initialize handler state with on-chain game account data.
    fn init_state(_context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        Ok(Self {
            messages: vec![],
            num_of_clients: init_account.players.len(),
            num_of_servers: init_account.servers.len(),
        })
    }

    /// Handle event.
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom { sender, raw } => {
                let event: GameEvent = serde_json::from_str(&raw).or(Err(Error::JsonParseError))?;
                match event {
                    GameEvent::PublicMessage { text } => {
                        self.messages.push(text);
                    }
                }
            }
            Event::Sync {
                new_players,
                new_servers,
                ..
            } => {
                self.num_of_clients += new_players.len();
                self.num_of_servers += new_servers.len();
            }
            Event::ServerLeave { .. } => {
                self.num_of_servers -= 1;
            }
            Event::Leave { .. } => {
                self.num_of_clients -= 1;
            }
            _ => (),
        }
        Ok(())
    }
}
