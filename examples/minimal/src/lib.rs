use race_api::prelude::*;
use race_proc_macro::game_handler;

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
    fn handle_custom_event(&mut self, event: MinimalEvent) -> Result<(), HandleError> {
        match event {
            MinimalEvent::Increment(n) => self.n += n,
        }
        Ok(())
    }
}

impl GameHandler for Minimal {

    fn init_state(init_account: InitAccount) -> Result<Self, HandleError> {
        let account_data: MinimalAccountData = init_account.data()?;
        Ok(Self {
            n: account_data.init_n,
        })
    }

    fn handle_event(&mut self, _effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            Event::Custom { raw, .. } => {
                let event = MinimalEvent::try_parse(&raw)?;
                self.handle_custom_event(event)
            }
            _ => Ok(()),
        }
    }

    fn balances(&self) -> Vec<PlayerBalance> {
        vec![]
    }
}
