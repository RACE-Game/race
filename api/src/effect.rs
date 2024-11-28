//! Effect for game handler

use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    engine::GameHandler,
    error::{HandleError, HandleResult},
    event::BridgeEvent,
    prelude::InitAccount,
    random::RandomSpec,
    types::{DecisionId, EntryLock, GamePlayer, RandomId, Settle, Transfer},
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Ask {
    pub player_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Assign {
    pub random_id: RandomId,
    pub player_id: u64,
    pub indices: Vec<usize>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Reveal {
    pub random_id: RandomId,
    pub indices: Vec<usize>,
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
    pub fn try_new<S: BorshSerialize, T: BorshSerialize>(
        id: usize,
        bundle_addr: String,
        max_players: u16,
        init_data: S,
    ) -> HandleResult<Self> {
        Ok(Self {
            id,
            bundle_addr,
            init_account: InitAccount {
                max_players,
                data: borsh::to_vec(&init_data)?,
            },
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct EmitBridgeEvent {
    pub dest: usize,
    pub raw: Vec<u8>,
}

impl EmitBridgeEvent {
    pub fn try_new<E: BridgeEvent>(dest: usize, bridge_event: E) -> HandleResult<Self> {
        Ok(Self {
            dest,
            raw: borsh::to_vec(&bridge_event)?,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct Log {
    pub level: LogLevel,
    pub message: String,
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
/// effect.assign(1 /* random_id */, 0 /* player_id */, vec![0, 1, 2] /* indices */);
/// ```
/// To reveal some items to the public, use [`Effect::reveal`].
/// It makes those items visible to everyone, including servers.
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// effect.reveal(1 /* random_id */, vec![0, 1, 2] /* indices */);
/// ```
///
/// # Decisions
///
/// To prompt a player for an hidden, immutable decision, use [`Effect::prompt`].
///
/// ```
/// # use race_api::effect::Effect;
/// let mut effect = Effect::default();
/// let decision_id = effect.ask(0 /* player_id */);
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
/// effect.settle(0 /* player_id */, 100 /* amount */, true /* eject */);
/// effect.checkpoint();
/// ```
///
/// # Reset
///
/// The game can be reset when there's no funds and pending deposit.
///
/// By resetting the game, all players will be removed and a new
/// checkpoint will be made.

#[derive(Default, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
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
    pub is_checkpoint: bool,
    pub settles: Vec<Settle>,
    pub handler_state: Option<Vec<u8>>,
    pub error: Option<HandleError>,
    pub transfers: Vec<Transfer>,
    pub launch_sub_games: Vec<SubGame>,
    pub bridge_events: Vec<EmitBridgeEvent>,
    pub valid_players: Vec<u64>,
    pub is_init: bool,
    pub entry_lock: Option<EntryLock>,
    pub reset: bool,
    pub logs: Vec<Log>,
}

impl Effect {
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
    pub fn assign(
        &mut self,
        random_id: RandomId,
        player_id: u64,
        indices: Vec<usize>,
    ) -> HandleResult<()> {
        self.assigns.push(Assign {
            random_id,
            player_id,
            indices,
        });
        Ok(())
    }

    /// Reveal some random items to the public.
    pub fn reveal(&mut self, random_id: RandomId, indices: Vec<usize>) {
        self.reveals.push(Reveal { random_id, indices })
    }

    /// Return the revealed random items by id.
    ///
    /// Return [`Error::RandomnessNotRevealed`] when invalid random id is given.
    pub fn get_revealed(&self, random_id: RandomId) -> HandleResult<&HashMap<usize, String>> {
        self.revealed
            .get(&random_id)
            .ok_or(HandleError::RandomnessNotRevealed)
    }

    /// Return the answer of a decision by id.
    ///
    /// Return [`Error::AnswerNotAvailable`] when invalid decision id
    /// is given or the answer is not ready.
    pub fn get_answer(&self, decision_id: DecisionId) -> HandleResult<&str> {
        if let Some(a) = self.answered.get(&decision_id) {
            Ok(a.as_ref())
        } else {
            Err(HandleError::AnswerNotAvailable)
        }
    }

    /// Ask a player for a decision, return the new decision id.
    pub fn ask(&mut self, player_id: u64) -> HandleResult<DecisionId> {
        self.asks.push(Ask { player_id });
        let decision_id = self.curr_decision_id;
        self.curr_decision_id += 1;
        Ok(decision_id)
    }

    pub fn release(&mut self, decision_id: DecisionId) {
        self.releases.push(Release { decision_id })
    }

    /// Dispatch action timeout event for a player after certain milliseconds.
    pub fn action_timeout(&mut self, player_id: u64, timeout: u64) -> HandleResult<()> {
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

    /// Set current state as the checkpoint.
    pub fn checkpoint(&mut self) {
        self.is_checkpoint = true;
    }

    /// Return if there's a checkpoint.
    pub fn is_checkpoint(&self) -> bool {
        self.is_checkpoint
    }

    /// Set the lock for entry.
    /// This will set current state as checkpoint automatically.
    pub fn set_entry_lock(&mut self, entry_lock: EntryLock) {
        self.checkpoint();
        self.entry_lock = Some(entry_lock);
    }

    /// Submit settlements.
    /// This will set current state as checkpoint automatically.
    pub fn settle(&mut self, player_id: u64, amount: u64, eject: bool) -> HandleResult<()> {
        self.checkpoint();
        self.settles.push(Settle::new(player_id, amount, eject));
        Ok(())
    }

    /// Transfer the assets to a recipient slot
    /// This will set current state as checkpoint automatically.
    pub fn transfer(&mut self, slot_id: u8, amount: u64) {
        self.checkpoint();
        self.transfers.push(Transfer { slot_id, amount });
    }

    /// Launches a new sub-game instance with specified parameters.
    ///
    /// # Parameters
    /// - `id`: Unique identifier for the sub-game. Must be non-zero.
    /// - `bundle_addr`: Address of the bundle associated with the sub-game.
    /// - `max_players`: Maximum number of players allowed in the sub-game.
    ///    The players in sub-game is managed by its master game, `max_players` is here for compatibility.
    /// - `init_data`: Initialization data for the sub-game. Must implement the `BorshSerialize` trait.
    ///
    /// # Returns
    /// - `Ok(())`: If the sub-game is successfully launched.
    /// - `Err(HandleError::InvalidSubGameId)`: If the provided sub-game `id` is zero.
    ///
    /// # Errors
    /// Returns an error if the initialization data cannot be serialized.
    pub fn launch_sub_game<D: BorshSerialize>(
        &mut self,
        id: usize,
        bundle_addr: String,
        max_players: u16,
        init_data: D,
    ) -> HandleResult<()> {
        if id == 0 {
            return Err(HandleError::InvalidSubGameId(id));
        }
        self.launch_sub_games.push(SubGame {
            id,
            bundle_addr,
            init_account: InitAccount {
                max_players,
                data: borsh::to_vec(&init_data)?,
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
        if let Ok(state) = borsh::to_vec(&handler_state) {
            self.handler_state = Some(state);
        } else {
            self.error = Some(HandleError::SerializationError);
        }
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
    pub fn bridge_event<E: BridgeEvent>(&mut self, dest: usize, evt: E) -> HandleResult<()> {
        if self.bridge_events.iter().any(|x| x.dest == dest) {
            return Err(HandleError::DuplicatedBridgeEventTarget);
        }

        self.bridge_events
            .push(EmitBridgeEvent::try_new(dest, evt)?);
        Ok(())
    }

    /// List bridge events, deserialize raw to event type E.
    pub fn list_bridge_events<E: BridgeEvent>(&self) -> HandleResult<Vec<(usize, E)>>{
        self.bridge_events.iter().map(|ref emit_bridge_event| {
            let dest = emit_bridge_event.dest;
            let event = E::try_from_slice(&emit_bridge_event.raw)?;
            Ok((dest, event))
        }).collect()
    }

    /// Reset the game to remove all players.  Be careful on usage,
    /// it only works when there's no funds left in game.
    /// A checkpoint will be made.
    pub fn reset(&mut self) {
        self.checkpoint();
        self.reset = true;
    }

    pub fn log<S: Into<String>>(&mut self, level: LogLevel, message: S) {
        self.logs.push(Log { level, message: message.into() });
    }

    pub fn info<S: Into<String>>(&mut self, message: S) {
        self.log(LogLevel::Info, message);
    }

    pub fn error<S: Into<String>>(&mut self, message: S) {
        self.log(LogLevel::Error, message);
    }

    pub fn warn<S: Into<String>>(&mut self, message: S) {
        self.log(LogLevel::Warn, message);
    }

    pub fn debug<S: Into<String>>(&mut self, message: S) {
        self.log(LogLevel::Debug, message);
    }
}

#[cfg(test)]
mod tests {}
