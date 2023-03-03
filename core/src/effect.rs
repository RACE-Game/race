//! Effect for game handler

use std::collections::BTreeMap;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    context::GameContext,
    engine::GameHandler,
    error::{Error, Result},
    random::RandomSpec,
    types::{DecisionId, RandomId, Settle},
};

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct Prompt {
    pub(crate) player_addr: String,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct Assign {
    pub(crate) random_id: RandomId,
    pub(crate) player_addr: String,
    pub(crate) indexes: Vec<usize>,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct Reveal {
    pub(crate) random_id: RandomId,
    pub(crate) indexes: Vec<usize>,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct InitRandomState {
    pub(crate) options: Vec<String>,
    pub(crate) size: usize,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub(crate) struct ActionTimeout {
    pub(crate) player_addr: String,
    pub(crate) timeout: u64,
}

/// An effect used in game handler, provide reading and mutating to
/// the game context.  An effect can be created from game context,
/// manipulated by game handler and applied after event processing.
///
/// # Num of Players and Servers
///
/// [`Effect::count_players`] and [`Effect::count_servers`] return the total number of
/// players and servers, which including those with pending status.
/// These functions are useful when detecting if there's enough
/// players/servers to statr a game.
///
/// # Randomness
///
/// To create a randomness, use [`Effect::init_random_state`] with a [`RandomSpec`].
///
/// ```
/// use race_core::random::deck_of_cards;
/// # use race_core::effect::Effect;
/// let mut effect = Effect::default();
/// let random_spec = deck_of_cards();
/// let random_id: usize = effect.init_random_state(&random_spec);
/// ```
///
/// To assign some items of the randomness to a specific player, use [`Effect::assign`].
/// It makes those items visible only to this player.
///
/// ```
/// # use race_core::effect::Effect;
/// let mut effect = Effect::default();
/// effect.assign(1 /* random_id */, "Alice", vec![0, 1, 2] /* indexes */);
/// ```
/// To reveal some items to public, use [`Effect::reveal`].
/// It makes those items visible to everyone(including servers).
///
/// ```
/// # use race_core::effect::Effect;
/// let mut effect = Effect::default();
/// effect.reveal(1 /* random_id */, vec![0, 1, 2] /* indexes */);
/// ```
///
/// # Decisions
///
/// To prompt a player for an hidden, immutable decision, use [`Effect::prompt`].
///
/// ```
/// # use race_core::effect::Effect;
/// let mut effect = Effect::default();
/// let decision_id: usize = effect.prompt("Alice");
/// ```
///
/// To reveal the answer, use [`Effect::reveal_answer`].
///
/// ```
/// # use race_core::effect::Effect;
/// let mut effect = Effect::default();
/// effect.reveal_answer(1 /* decision_id */);
/// ```
///
/// # Timeouts
///
/// Two types of timeout event can be dispatched: `action_timeout` and
/// `wait_timeout`.
///
/// - Action Timeout:
/// Represent a player doesn't act in time, a player address is
/// required in this case.
///
/// - Wait Timeout:
/// Represent a general waiting. It's useful when you want to start a
/// game in a certain timeout, regardless of how many players are
/// available.
///
/// # Settle
///
/// Add settlements with [`Effect::settle`].
///
/// ```
/// # use race_core::effect::Effect;
/// use race_core::types::Settle;
/// let mut effect = Effect::default();
/// // Increase assets
/// effect.settle(Settle::add("Alice", 100));
/// // Decrease assets
/// effect.settle(Settle::sub("Bob", 200));
/// // Remove player from this game, its assets will be paid out
/// effect.settle(Settle::eject("Charlie"));
/// ```

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct Effect {
    pub(crate) action_timeout: Option<ActionTimeout>,
    pub(crate) wait_timeout: Option<u64>,
    pub(crate) start_game: bool,
    pub(crate) stop_game: bool,
    pub(crate) cancel_dispatch: bool,
    pub(crate) timestamp: u64,
    pub(crate) curr_random_id: RandomId,
    pub(crate) curr_decision_id: DecisionId,
    pub(crate) players_count: usize,
    pub(crate) servers_count: usize,
    pub(crate) prompts: Vec<Prompt>,
    pub(crate) assigns: Vec<Assign>,
    pub(crate) reveals: Vec<Reveal>,
    pub(crate) init_random_states: Vec<InitRandomState>,
    pub(crate) revealed: BTreeMap<usize, BTreeMap<usize, String>>,
    pub(crate) answered: BTreeMap<usize, String>,
    pub(crate) settles: Vec<Settle>,
    pub(crate) handler_state: Option<Vec<u8>>,
    pub(crate) error: Option<Error>,
    pub(crate) allow_exit: bool,
}

impl Effect {
    pub fn from_context(context: &GameContext) -> Self {
        let revealed = context
            .list_random_states()
            .iter()
            // Switch to BTreeMap
            .map(|st| (st.id, BTreeMap::from_iter(st.revealed.clone().into_iter())))
            .collect();

        let answered = context
            .list_decision_states()
            .iter()
            .filter_map(|st| {
                if let Some(a) = st.get_revealed() {
                    Some((st.id, a.to_owned()))
                } else {
                    None
                }
            })
            .collect();

        Self {
            start_game: false,
            stop_game: false,
            cancel_dispatch: false,
            action_timeout: None,
            wait_timeout: None,
            timestamp: context.timestamp,
            curr_random_id: context.list_random_states().len(),
            curr_decision_id: context.list_decision_states().len(),
            players_count: context.count_players(),
            servers_count: context.count_servers(),
            prompts: Vec::new(),
            assigns: Vec::new(),
            reveals: Vec::new(),
            init_random_states: Vec::new(),
            revealed,
            answered,
            settles: Vec::new(),
            handler_state: None,
            error: None,
            allow_exit: context.allow_exit,
        }
    }

    /// Return the number of players, including both pending and joint.
    pub fn count_players(&self) -> usize {
        self.players_count
    }

    /// Return the number of servers, including both pending and joint.
    pub fn count_servers(&self) -> usize {
        self.servers_count
    }

    /// Initialize a random state with random spec, return random id.
    pub fn init_random_state(&mut self, random_spec: &dyn RandomSpec) -> RandomId {
        let options = random_spec.options().clone();
        let size = random_spec.size();
        self.init_random_states
            .push(InitRandomState { options, size });
        let random_id = self.curr_random_id;
        self.curr_random_id += 1;
        random_id
    }

    /// Assign some random items to a specific player.
    pub fn assign<S: Into<String>>(
        &mut self,
        random_id: RandomId,
        player_addr: S,
        indexes: Vec<usize>,
    ) {
        self.assigns.push(Assign {
            random_id,
            player_addr: player_addr.into(),
            indexes,
        })
    }

    /// Reveal some random items to public.
    pub fn reveal(&mut self, random_id: RandomId, indexes: Vec<usize>) {
        self.reveals.push(Reveal { random_id, indexes })
    }

    /// Return the revealed random items by id.
    ///
    /// Return [`Error::InvalidRandomId`] when invalid random id is given.
    pub fn get_revealed(&self, random_id: RandomId) -> Result<&BTreeMap<usize, String>> {
        self.revealed.get(&random_id).ok_or(Error::InvalidRandomId)
    }

    /// Return the answer of a decision by id.
    ///
    /// Return [`Error::AnswerNotAvailable`] when invalid decision id
    /// is given or the answer is not ready.
    pub fn get_answer(&self, decision_id: DecisionId) -> Result<&str> {
        if let Some(a) = self.answered.get(&decision_id) {
            Ok(a.as_ref())
        } else {
            Err(Error::AnswerNotAvailable)
        }
    }

    /// Prompt a player for a decision, return the new decision id.
    pub fn prompt<S: Into<String>>(&mut self, player_addr: S) -> DecisionId {
        self.prompts.push(Prompt {
            player_addr: player_addr.into(),
        });
        let decision_id = self.curr_decision_id;
        self.curr_decision_id += 1;
        decision_id
    }

    /// Reveal an answer of a decision by id.
    pub fn reveal_answer(&mut self, _decision_id: DecisionId) {}

    /// Dispatch action timeout event for a player after certain milliseconds.
    pub fn action_timeout<S: Into<String>>(&mut self, player_addr: S, timeout: u64) {
        self.action_timeout = Some(ActionTimeout {
            player_addr: player_addr.into(),
            timeout,
        });
    }

    /// Return current timestamp.
    ///
    /// The event handling must be pure, so it's not allowed to use
    /// timestamp from system API.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Dispatch waiting timeout event after certain milliseconds.
    pub fn wait_timeout(&mut self, timeout: u64) {
        self.wait_timeout = Some(timeout);
    }

    /// Start the game.
    pub fn start_game(&mut self) {
        self.start_game = true;
    }

    /// Stop the game.
    pub fn stop_game(&mut self) {
        self.stop_game = true;
    }

    /// Set if exiting game is allowed.
    pub fn allow_exit(&mut self, allow_exit: bool) {
        self.allow_exit = allow_exit
    }

    /// Submit settlements.
    pub fn settle(&mut self, settle: Settle) {
        self.settles.push(settle);
    }

    /// Get handler state.
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __handler_state<S>(&self) -> S
    where
        S: GameHandler,
    {
        S::try_from_slice(self.handler_state.as_ref().unwrap()).unwrap()
    }

    /// Set handler state.
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __set_handler_state<S: BorshSerialize>(&mut self, handler_state: S) {
        if let Ok(state) = handler_state.try_to_vec() {
            self.handler_state = Some(state);
        } else {
            self.error = Some(Error::SerializationError);
        }
    }

    /// Set error.
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __set_error(&mut self, error: Error) {
        self.error = Some(error);
    }

    /// Take error
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __take_error(&mut self) -> Option<Error> {
        std::mem::replace(&mut self.error, None)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_serialization() -> anyhow::Result<()> {
        let mut answered = BTreeMap::new();
        answered.insert(1, "A".into());

        let mut revealed = BTreeMap::new();
        {
            let mut m = BTreeMap::new();
            m.insert(1, "B".into());
            revealed.insert(2, m);
        }

        let effect = Effect {
            action_timeout: Some(ActionTimeout {
                player_addr: "alice".into(),
                timeout: 100,
            }),
            wait_timeout: Some(200),
            start_game: true,
            stop_game: true,
            cancel_dispatch: true,
            timestamp: 300_000,
            curr_random_id: 1,
            curr_decision_id: 1,
            players_count: 4,
            servers_count: 4,
            prompts: vec![Prompt {
                player_addr: "bob".into(),
            }],
            assigns: vec![Assign {
                player_addr: "bob".into(),
                random_id: 1,
                indexes: vec![0, 1, 2],
            }],
            reveals: vec![Reveal {
                random_id: 1,
                indexes: vec![0, 1, 2],
            }],
            init_random_states: vec![InitRandomState {
                options: vec!["a".into(), "b".into()],
                size: 2,
            }],
            revealed,
            answered,
            settles: vec![Settle::add("alice", 200), Settle::add("bob", 200)],
            handler_state: Some(vec![0, 1, 2, 3]),
            error: Some(Error::NoEnoughPlayers),
            allow_exit: true,
        };
        let bs = effect.try_to_vec()?;

        let parsed = Effect::try_from_slice(&bs)?;
        assert_eq!(effect, parsed);
        assert!(false);
        Ok(())
    }
}
