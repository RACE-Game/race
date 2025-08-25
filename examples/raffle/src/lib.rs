//! A simple raffle example
//!
//! This example demonstrates randomness handling within the game bundle.
//!
//! Game Rules:
//!
//! Players can join at any time. Once at least one player participates,
//! a raffle is scheduled to run in one minute.  Each newly joined player extends this scheduling.
//! However, if fewer than two players remain when the timer ends, the raffle is cancelled.
//!
//! The raffle selects a winner by randomly selecting a player ID.
//!
//! After each run, all players are removed from the game.
//!
//! Players joining mid-raffle are eligible for the next round.

use race_api::prelude::*;
use race_proc_macro::game_handler;

const DRAW_TIMEOUT: u64 = 60_000;
const END_TIMEOUT: u64 = 10_000;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
enum PlayerStatus {
    /// Just joined, the deposit is not ready yet.
    /// This player is ineligible until its deposit confirms.
    Init,

    /// The deposit confirms.
    /// The player is eligible for raffle.
    Ready,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
struct Player {
    /// The player id, assigned by the protocol.
    pub id: u64,
    /// The deposit, since the ticket is fixed in this game
    /// Every player has the same `balance`.
    pub balance: u64,
    /// whether this player is eligible for raffle.
    pub status: PlayerStatus,
}

impl From<GamePlayer> for Player {
    fn from(value: GamePlayer) -> Self {
        Player { id: value.id(), balance: 0,  status: PlayerStatus::Init }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
#[game_handler]
struct Raffle {
    winner_player_id: Option<u64>,
    players: Vec<Player>,
    random_id: RandomId,  // We save random id, and we use it to get randomness information in the game progress.
    draw_time: u64,
    prize_pool: u64,
    max_players: u16, // The maximum number of players supported in this game.
}

impl GameHandler for Raffle {

    /// Initialize handler state with on-chain game account data.
    /// Set basic behaviors with effect, in this case, the entry lock.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self> {
        // In this game, we allow buyin & deposit, but disallow deposit-only.
        effect.set_entry_lock(EntryLock::JoinOnly);
        Ok(Self {
            winner_player_id: None,
            players: vec![],
            random_id: 0,
            draw_time: 0,
            prize_pool: 0,
            max_players: init_account.max_players,
        })
    }

    /// Report token distribution to the blockchain.  This ensures accurate accounting
    /// and serves as the definitive record for handling tokens in disrupted games.
    fn balances(&self) -> Vec<PlayerBalance> {
        self.players.iter().map(|p| PlayerBalance::new(p.id, p.balance)).collect()
    }

    /// The main event handler.  All game logic goes here.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
        match event {

            // Event emitted when a player joins.
            // We don't check `max_players` here, as overflowed join
            // attemps are rejected by the contract.
            Event::Join { players } => {
                let players = players.into_iter().map(Into::into);
                self.players.extend(players);
            }

            // Event emitted when a deposit confirms.
            // This event comes right after the Join event.
            // Because we set EntryType to JoinOnly,
            // Join & Deposit can be considered as an atomic operation.
            //
            // So, in reality, we don't really have players with Init status.
            // If independent deposit is allowed, invalid deposits must be
            // handled properly.
            //
            // Deposit amount validation can be handled by using
            // EntryType::Ticket, allowing contract to perform the check.
            Event::Deposit { deposits } => {
                for d in deposits {
                    let Some(player) = self.players.iter_mut().find(|p| p.id == d.id()) else {
                        // this never happens
                        effect.reject_deposit(&d)?;
                        return Ok(());
                    };
                    player.status = PlayerStatus::Ready;
                    if self.players.len() >= 1 {
                        self.draw_time = effect.timestamp() + DRAW_TIMEOUT;
                        // We wait one minute, then see how many
                        // players we have in game.
                        effect.wait_timeout(DRAW_TIMEOUT);
                    }
                }
            }

            // One-minute wait completed or game is ended(see SecretsReady).
            Event::WaitingTimeout => {
                // The game is ended, the result has been displayed
                // We should reset the game
                if self.winner_player_id.is_some() {
                    self.cleanup(effect);
                } else if self.players.len() > 1 {
                    // Start raffle if we have enough players
                    // or cancel the game if we don't.
                    effect.start_game();
                } else {
                    self.cancel_game(effect);
                }
            }

            // Event emitted when Effect::start_game is called.
            // This event will reset all randomness state.
            Event::GameStart => {
                // We collect all player IDs as options.
                let options = self.players.iter().map(|p| p.id.to_string()).collect();
                // A random specification for shuffling these options.
                let rnd_spec = RandomSpec::shuffled_list(options);
                // Ask the protocol to initialize the randomness.
                // We save the random ID for future use.
                self.random_id = effect.init_random_state(rnd_spec);
            }

            // Event emitted when the randomness is generated.
            Event::RandomnessReady { .. } => {
                // In our raffle, we select one winner by
                // taking the first item in the list as the
                // winner's player ID.
                //
                // Note: when this event is emitted, the randomness is just ready but not
                // yet reaveled.  To reveal it, we must ask the protocol
                // to share secrets first.
                effect.reveal(self.random_id, vec![0]);
            }

            // Event emitted when requested secrets are shared.
            // It means now we can read the result of randomization.
            Event::SecretsReady { .. } => {
                // Take the first item as the winner's player ID
                let winner = effect
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                for p in self.players.iter() {
                    if p.id == winner {
                        // Transfer the token to the winner
                        effect.withdraw(p.id, self.prize_pool);
                    }
                }
                effect.checkpoint();
                // Save the winner's ID for display in frontend.
                self.winner_player_id = Some(winner);
                // Schedule a timeout, give frontend some secs to display the raffle result.
                effect.wait_timeout(END_TIMEOUT);
            }

            // Event emitted when some server is down
            // We simply cancel the game.
            Event::OperationTimeout { .. } => {
                self.cancel_game(effect);
            }

            _ => (),
        }
        Ok(())
    }
}

impl Raffle {
    /// Remove all players from game and reset every thing, prepare for next round.
    fn cleanup(&mut self, effect: &mut Effect) {
        for p in self.players.iter() {
            effect.eject(p.id);
        }
        self.winner_player_id = None;
        self.players.clear();
        self.random_id = 0;
        self.draw_time = 0;
        self.prize_pool = 0;
    }

    /// Cancel the game and refund players
    fn cancel_game(&mut self, effect: &mut Effect) {
        for p in self.players.iter() {
            // Refund the player
            effect.withdraw(p.id, p.balance);
        }
        self.cleanup(effect);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use race_test::prelude::*;
    use super::*;

    #[test]
    fn test_game_flow() -> anyhow::Result<()> {
        let mut tx = TestClient::transactor("tx");
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let (mut ctx, _) = TestContextBuilder::default()
            .with_max_players(10)
            .set_transactor(&mut tx)
            .with_deposit_amount(1000)
            .build_with_init_state::<Raffle>()?;

        let (join_event, deposit_event) = ctx.join_multi(vec![
            (&mut alice, 1000),
            (&mut bob, 1000),
        ]);

        ctx.handle_multiple_events(&[join_event, deposit_event])?;

        let dispatch = ctx.current_dispatch();

        // After enough players joined, we expect there's a waiting timeout event being dispatched
        assert_eq!(dispatch, Some(DispatchEvent::new(Event::WaitingTimeout, DRAW_TIMEOUT)));

        // We handle the dispatched event and all events after it, stop right before the SecretsReady
        let (secrets_ready, _) = ctx.handle_dispatch_until(vec![&mut alice, &mut bob, &mut tx], |e| matches!(e, Some(&Event::SecretsReady {..})))?;

        let random_id = ctx.state().random_id;

        // Before process the dispatching events, we set a faked random result
        // We select bob as the winner.
        ctx.set_random_result(random_id, HashMap::from([(0, alice.id().to_string())]));

        ctx.handle_event_until_no_events(&secrets_ready.unwrap(), vec![&mut alice, &mut bob, &mut tx])?;
        {
            let state = ctx.state();
            assert_eq!(state.winner_player_id, Some(alice.id()));
        }

        Ok(())
    }
}
