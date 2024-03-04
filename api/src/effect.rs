//! Effect for game handler

use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    engine::GameHandler,
    error::{Error, HandleError, Result},
    event::BridgeEvent,
    prelude::InitAccount,
    random::RandomSpec,
    types::{DecisionId, GamePlayer, RandomId, Settle, Transfer},
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Ask {
    pub player_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Assign {
    pub random_id: RandomId,
    pub player_id: u64,
    pub indexes: Vec<usize>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Reveal {
    pub random_id: RandomId,
    pub indexes: Vec<usize>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Release {
    pub decision_id: DecisionId,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct ActionTimeout {
    pub player_id: u64,
    pub timeout: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct SubGame {
    pub id: usize,
    pub bundle_addr: String,
    pub init_account: InitAccount,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct SubGameJoin {
    pub id: usize,
    pub players: Vec<GamePlayer>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct SubGameLeave {
    pub id: usize,
    pub player_ids: Vec<u64>,
}

impl SubGame {
    pub fn try_new<S: BorshSerialize>(
        id: usize,
        bundle_addr: String,
        max_players: u16,
        players: Vec<GamePlayer>,
        init_data: S,
        checkpoint: S,
    ) -> Result<Self> {
        Ok(Self {
            id,
            bundle_addr,
            init_account: InitAccount {
                max_players,
                entry_type: crate::types::EntryType::Disabled,
                players,
                data: init_data.try_to_vec()?,
                checkpoint: checkpoint.try_to_vec()?,
            },
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct EmitBridgeEvent {
    pub dest: usize,
    pub raw: Vec<u8>,
    pub join_players: Vec<GamePlayer>,
}

impl EmitBridgeEvent {
    pub fn try_new<E: BridgeEvent>(
        dest: usize,
        bridge_event: E,
        join_players: Vec<GamePlayer>,
    ) -> Result<Self> {
        Ok(Self {
            dest,
            raw: bridge_event.try_to_vec()?,
            join_players,
        })
    }
}

/// An effect used in game handler provides reading and mutating to
/// the game context.  An effect can be created from game context,
/// manipulated by game handler and applied after event processing.
///
/// # Num of Players and Servers
///
/// [`Effect::count_players`] and [`Effect::count_servers`] return the total number of
/// players and servers, respectively. The number includes those with pending status.
/// These functions are useful when detecting if there's enough players/servers for
/// a game to start.
///
/// # Randomness
///
/// To create a randomness, use [`Effect::init_random_state`] with a [`RandomSpec`].
///
/// ```
/// # use race_api::effect::Effect;
/// use race_api::random::RandomSpec;
/// let mut effect = Effect::default();
/// let random_spec = RandomSpec::deck_of_cards();
/// let random_id = effect.init_random_state(random_spec);
/// ```
///
/// To assign some items of the randomness to a specific player, use [`Effect::assign`].
/// It makes those items visible only to this player.
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// effect.assign(1 /* random_id */, "Alice", vec![0, 1, 2] /* indexes */);
/// ```
/// To reveal some items to the public, use [`Effect::reveal`].
/// It makes those items visible to everyone, including servers.
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// effect.reveal(1 /* random_id */, vec![0, 1, 2] /* indexes */);
/// ```
///
/// # Decisions
///
/// To prompt a player for an hidden, immutable decision, use [`Effect::prompt`].
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// let decision_id = effect.ask("Alice");
/// ```
///
/// To reveal the answer, use [`Effect::reveal_answer`].
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// effect.release(1 /* decision_id */);
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
/// # use race_api::effect::Effect;
/// use race_api::types::Settle;
/// let mut effect = Effect::default();
/// // Increase assets
/// effect.settle(Settle::add("Alice", 100));
/// // Decrease assets
/// effect.settle(Settle::sub("Bob", 200));
/// // Remove player from this game, its assets will be paid out
/// effect.settle(Settle::eject("Charlie"));
/// // Make the checkpoint
/// effect.checkpoint();
/// ```

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshSerialize, BorshDeserialize, Debug)]
pub struct Effect {
    pub action_timeout: Option<ActionTimeout>,
    pub wait_timeout: Option<u64>,
    pub start_game: bool,
    pub stop_game: bool,
    pub cancel_dispatch: bool,
    pub timestamp: u64,
    pub curr_random_id: RandomId,
    pub curr_decision_id: DecisionId,
    pub nodes_count: u16,
    pub asks: Vec<Ask>,
    pub assigns: Vec<Assign>,
    pub reveals: Vec<Reveal>,
    pub releases: Vec<Release>,
    pub init_random_states: Vec<RandomSpec>,
    pub revealed: HashMap<RandomId, HashMap<usize, String>>,
    pub answered: HashMap<DecisionId, String>,
    pub checkpoint: Option<Vec<u8>>,
    pub settles: Vec<Settle>,
    pub handler_state: Option<Vec<u8>>,
    pub error: Option<HandleError>,
    pub allow_exit: bool,
    pub transfers: Vec<Transfer>,
    pub launch_sub_games: Vec<SubGame>,
    pub bridge_events: Vec<EmitBridgeEvent>,
    pub valid_players: Vec<GamePlayer>,
}

impl Effect {

    fn assert_player_id(&self, id: u64) -> Result<()> {
        if self.valid_players.iter().find(|p| p.id == id).is_some() {
           Ok(())
        } else {
           Err(Error::InvalidPlayerId(id, self.valid_players.iter().map(|p| p.id).collect()))
        }
    }

    /// Return the number of nodes, including both the pending and joined.
    pub fn count_nodes(&self) -> usize {
        self.nodes_count as usize
    }

    /// Initialize a random state with random spec, return random id.
    pub fn init_random_state(&mut self, spec: RandomSpec) -> RandomId {
        self.init_random_states.push(spec);
        let random_id = self.curr_random_id;
        self.curr_random_id += 1;
        random_id
    }

    /// Assign some random items to a specific player.
    pub fn assign(&mut self, random_id: RandomId, player_id: u64, indexes: Vec<usize>) -> Result<()> {
        self.assert_player_id(player_id)?;
        self.assigns.push(Assign {
            random_id,
            player_id,
            indexes,
        });
        Ok(())
    }

    /// Reveal some random items to the public.
    pub fn reveal(&mut self, random_id: RandomId, indexes: Vec<usize>) {
        self.reveals.push(Reveal { random_id, indexes })
    }

    /// Return the revealed random items by id.
    ///
    /// Return [`Error::RandomnessNotRevealed`] when invalid random id is given.
    pub fn get_revealed(&self, random_id: RandomId) -> Result<&HashMap<usize, String>> {
        self.revealed
            .get(&random_id)
            .ok_or(Error::RandomnessNotRevealed)
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

    /// Ask a player for a decision, return the new decision id.
    pub fn ask(&mut self, player_id: u64) -> Result<DecisionId> {
        self.assert_player_id(player_id)?;
        self.asks.push(Ask { player_id });
        let decision_id = self.curr_decision_id;
        self.curr_decision_id += 1;
        Ok(decision_id)
    }

    pub fn release(&mut self, decision_id: DecisionId) {
        self.releases.push(Release { decision_id })
    }

    /// Dispatch action timeout event for a player after certain milliseconds.
    pub fn action_timeout(&mut self, player_id: u64, timeout: u64) -> Result<()> {
        self.assert_player_id(player_id)?;
        self.action_timeout = Some(ActionTimeout { player_id, timeout });
        Ok(())
    }

    /// Return current timestamp.
    ///
    /// The event handling must be pure, so it's not allowed to use
    /// timestamp from system API.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Cancel current dispatched event.
    pub fn cancel_dispatch(&mut self) {
        self.cancel_dispatch = true;
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

    /// Set checkpoint can trigger settlements.
    pub fn checkpoint<S: BorshSerialize>(&mut self, checkpoint_state: S) {
        if let Ok(checkpoint) = checkpoint_state.try_to_vec() {
            self.checkpoint = Some(checkpoint);
        } else {
            self.error = Some(HandleError::SerializationError)
        }
    }

    pub fn is_checkpoint(&self) -> bool {
        self.checkpoint.is_some()
    }

    /// Submit settlements.
    pub fn settle(&mut self, settle: Settle) -> Result<()> {
        self.assert_player_id(settle.id)?;
        self.settles.push(settle);
        Ok(())
    }

    /// Transfer the assets to a recipient slot
    pub fn transfer(&mut self, slot_id: u8, amount: u64) {
        self.transfers.push(Transfer { slot_id, amount });
    }

    /// Launch sub game
    pub fn launch_sub_game<D: BorshSerialize, C: BorshSerialize>(
        &mut self,
        id: usize,
        bundle_addr: String,
        max_players: u16,
        players: Vec<GamePlayer>,
        init_data: D,
        checkpoint: C,
    ) -> Result<()> {
        for p in players.iter() {
            self.assert_player_id(p.id)?;
        }

        self.launch_sub_games.push(SubGame {
            id,
            bundle_addr,
            init_account: InitAccount {
                max_players,
                entry_type: crate::types::EntryType::Disabled,
                players,
                data: init_data.try_to_vec()?,
                checkpoint: checkpoint.try_to_vec()?,
            },
        });
        Ok(())
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
            self.error = Some(HandleError::SerializationError);
        }
    }

    pub fn __set_checkpoint_raw(&mut self, raw: Vec<u8>) {
        self.checkpoint = Some(raw);
    }

    pub fn __checkpoint(&mut self) -> Option<Vec<u8>> {
        self.checkpoint.take()
    }

    /// Set error.
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __set_error(&mut self, error: HandleError) {
        self.error = Some(error);
    }

    /// Take error
    ///
    /// This is an internal function, DO NOT use in game handler.
    pub fn __take_error(&mut self) -> Option<HandleError> {
        self.error.take()
    }

    /// Emit a bridge event.
    pub fn bridge_event<E: BridgeEvent>(
        &mut self,
        dest: usize,
        evt: E,
        join_players: Vec<GamePlayer>,
    ) -> Result<()> {
        for p in join_players.iter() {
            self.assert_player_id(p.id)?;
        }

        self.bridge_events
            .push(EmitBridgeEvent::try_new(dest, evt, join_players)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_serialization() -> anyhow::Result<()> {
        let mut answered = HashMap::new();
        answered.insert(33, "A".into());

        let mut revealed = HashMap::new();
        {
            let mut m = HashMap::new();
            m.insert(11, "B".into());
            revealed.insert(22, m);
        }

        let effect = Effect {
            action_timeout: Some(ActionTimeout {
                player_id: 0,
                timeout: 100,
            }),
            wait_timeout: Some(200),
            start_game: true,
            stop_game: true,
            cancel_dispatch: true,
            timestamp: 300_000,
            curr_random_id: 1,
            curr_decision_id: 1,
            nodes_count: 4,
            asks: vec![Ask { player_id: 1 }],
            assigns: vec![Assign {
                player_id: 1,
                random_id: 5,
                indexes: vec![0, 1, 2],
            }],
            reveals: vec![Reveal {
                random_id: 6,
                indexes: vec![0, 1, 2],
            }],
            releases: vec![Release { decision_id: 7 }],
            init_random_states: vec![RandomSpec::shuffled_list(vec!["a".into(), "b".into()])],
            revealed,
            answered,
            settles: vec![Settle::add(0, 200), Settle::sub(1, 200)],
            handler_state: Some(vec![1, 2, 3, 4]),
            error: Some(HandleError::NoEnoughPlayers),
            allow_exit: true,
            transfers: vec![],
            checkpoint: None,
            launch_sub_games: vec![],
            bridge_events: vec![],
            valid_players: vec![],
        };
        let bs = effect.try_to_vec()?;

        let parsed = Effect::try_from_slice(&bs)?;

        assert_eq!(effect, parsed);
        Ok(())
    }
}
