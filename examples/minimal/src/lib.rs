use race_api::prelude::*;
use race_proc_macro::game_handler;

#[derive(BorshDeserialize, BorshSerialize)]
enum MinimalEvent {
    Increment(u64),
}

impl CustomEvent for MinimalEvent {}

#[derive(BorshSerialize, BorshDeserialize)]
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

    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> Result<Self, HandleError> {
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


#[cfg(test)]
mod tests {

    use race_test::prelude::*;
    use super::*;

    #[test]
    fn test_random_state() {

        let mut transactor = TestClient::transactor("tx");
        let mut alice = TestClient::player("alice");

        let (mut ctx, _) = TestContextBuilder::default()
            .with_max_players(10)
            .with_deposit_range(100, 200)
            .with_data(MinimalAccountData { init_n: 1 })
            .set_transactor(&mut transactor)
            .add_player(&mut alice, 100)
            .build_with_init_state::<Minimal>().unwrap();

        {
            assert_eq!(ctx.state().n, 1);
        }

        let e = alice.custom_event(MinimalEvent::Increment(10));
        ctx.handle_event(&e).unwrap();

        {
            assert_eq!(ctx.state().n, 11);
        }
    }
}
