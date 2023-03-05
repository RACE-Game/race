use race_core::prelude::*;

#[derive(BorshDeserialize, BorshSerialize)]
enum MinimalEvent {
    Increment(u64),
}

impl CustomEvent for MinimalEvent {}

#[derive(BorshDeserialize)]
struct MinimalAccountData {
    init_n: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[game_handler]
struct Minimal {
    n: u64,
}

impl Minimal {
    fn handle_custom_event(&mut self, event: MinimalEvent) -> Result<()> {
        match event {
            MinimalEvent::Increment(n) => self.n += n,
        }
        Ok(())
    }
}

impl GameHandler for Minimal {
    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> Result<Self> {
        let account_data: MinimalAccountData = init_account.data()?;
        Ok(Self {
            n: account_data.init_n,
        })
    }

    fn handle_event(&mut self, _effect: &mut Effect, event: Event) -> Result<()> {
        match event {
            Event::Custom { raw, .. } => {
                let event = MinimalEvent::try_from_slice(&raw)?;
                self.handle_custom_event(event)
            }
            _ => Ok(()),
        }
    }
}
