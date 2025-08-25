///! This example shows the minimal setup for a game bundle.
///!
///! A simple counter for the number of players in game.
///!
///! When a player joins, counter increases by 1.
///! When a player lefts, counter decreases by 1.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use race_api::prelude::*;
use race_proc_macro::game_handler;

///! This bundle needs no properties for initialization.
#[derive(BorshSerialize, BorshDeserialize)]
struct MinimalAccountData { }

#[derive(BorshDeserialize, BorshSerialize, Default)]
#[game_handler]
struct Minimal {
    /// A map from player ID to its balance.
    ///
    /// Note: Although we don't care about balances in this example,
    /// but, in order to have accounting works correct on chain,
    /// Implementing balances function is required.
    player_balances: BTreeMap<u64, u64>,

    /// The number of players in game.
    num_of_players: usize,
}

impl GameHandler for Minimal {

    fn init_state(effect: &mut Effect, _init_account: InitAccount) -> Result<Self, HandleError> {
        effect.set_entry_lock(EntryLock::JoinOnly);
        Ok(Default::default())
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            // Event emitted when players joined.
            Event::Join { players } => {
                self.num_of_players += players.len();
            }

            // Event emitted when deposits confirmed.
            Event::Deposit { deposits } => {
                deposits.into_iter().for_each(|d| {
                    self.player_balances.insert(d.id(), d.balance());
                })
            }

            // Event emitted when player leaves.
            Event::Leave { player_id } => {
                match self.player_balances.entry(player_id) {
                    Entry::Occupied(e) => {
                        effect.withdraw(player_id, *e.get());
                        effect.eject(player_id);
                        e.remove();
                        self.num_of_players -= 1;
                    }
                    Entry::Vacant(_) => {
                        return Err(HandleError::InvalidPlayer);
                    }
                }

            }
            _ => (),
        }
        Ok(())
    }

    fn balances(&self) -> Vec<PlayerBalance> {
        self.player_balances.iter().map(|(id, balance)| PlayerBalance::new(*id, *balance)).collect()
    }
}


#[cfg(test)]
mod tests {

    use race_test::prelude::*;
    use super::*;
    #[test]
    fn test_random_state() -> anyhow::Result<()> {

        let mut transactor = TestClient::transactor("tx");
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let mut carol = TestClient::player("carol");

        let (mut ctx, _) = TestContextBuilder::default()
            .with_max_players(10)
            .with_deposit_range(100, 200)
            .set_transactor(&mut transactor)
            .build_with_init_state::<Minimal>()?;

        {
            assert_eq!(ctx.state().num_of_players, 0);
        }

        let (join_event, deposit_event) = ctx.join_multi(vec![
            (&mut alice, 100),
            (&mut bob, 150),
            (&mut carol, 200),
        ]);

        ctx.handle_multiple_events(&[join_event, deposit_event])?;

        assert_eq!(ctx.state().num_of_players, 3);

        let leave_event = ctx.leave(&alice);

        ctx.handle_event(&leave_event)?;

        assert_eq!(ctx.state().num_of_players, 2);

        Ok(())
    }
}
