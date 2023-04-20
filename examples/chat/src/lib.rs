use race_core::prelude::*;

#[derive(Serialize, Deserialize)]
enum GameEvent {
    Message { text: String },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
struct Message {
    sender: String,
    text: String,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[game_handler]
struct Chat {
    messages: Vec<Message>,
}

impl GameHandler for Chat {
    /// Initialize handler state with on-chain game account data.
    fn init_state(context: &mut Effect, _init_account: InitAccount) -> Result<Self> {
        Ok(Self { messages: vec![] })
    }

    /// Handle event.
    fn handle_event(&mut self, _context: &mut Effect, event: Event) -> Result<()> {
        match event {
            Event::Custom { sender, raw } => {
                let event: GameEvent = serde_json::from_str(&raw).or(Err(Error::JsonParseError))?;
                match event {
                    GameEvent::Message { text } => {
                        self.messages.push(Message { sender, text });
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
}
