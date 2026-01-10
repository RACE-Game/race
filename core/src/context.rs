use crate::checkpoint::{VersionedData, SharedData, ContextCheckpoint};
use crate::decision::DecisionState;
use crate::error::{Error, Result};
use crate::versions::Versions;
use crate::random::{RandomState, RandomStatus};
use crate::types::ClientMode;
use crate::entry_type::EntryType;
use crate::game_spec::GameSpec;
use crate::dispatch_event::DispatchEvent;
use crate::node::{Node, NodeStatus};
use race_api::effect::{
    Ask, Assign, Effect, EmitBridgeEvent, Log, Release, Reveal, LaunchSubGame, Withdraw,
};
use race_api::engine::GameHandler;
use race_api::event::{CustomEvent, Event};
use race_api::random::RandomSpec;
use race_api::types::{
    Award, BalanceChange, Ciphertext, EntryLock, GameStatus,
    PlayerBalance, SecretDigest, SecretShare, Settle, Transfer,
};
use sha256::digest;
use std::collections::HashMap;
use std::mem::take;

const OPERATION_TIMEOUT: u64 = 15_000;

/// The effects of an event, indicates what actions should be taken
/// after the event handling.
///
/// - checkpoint: to send a settlement.
/// - launch_sub_games: to launch a list of sub games.
/// - bridge_events: to send events to sub games.
/// - start_game: to start game.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EventEffects {
    pub launch_sub_games: Vec<LaunchSubGame>,
    pub bridge_events: Vec<EmitBridgeEvent>,
    pub start_game: bool,
    pub stop_game: bool,
    pub logs: Vec<Log>,
    pub reject_deposits: Vec<u64>,
    pub checkpoint: Option<ContextCheckpoint>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettleDetails {
    pub settles: Vec<Settle>,
    pub transfer: Option<Transfer>,
    pub awards: Vec<Award>,
    pub checkpoint: ContextCheckpoint,
    pub access_version: u64,
    pub settle_version: u64,
    pub previous_settle_version: u64,
    pub entry_lock: Option<EntryLock>,
    pub accept_deposits: Vec<u64>,
    // The locks are identified by a version number.
    pub settle_locks: HashMap<usize, u64>,
}

impl SettleDetails {
    pub fn handle_versioned_data(&mut self, versioned_data: VersionedData) -> Result<()> {
        self.settle_locks.remove(&versioned_data.game_spec.game_id);
        if self.checkpoint.root_data.sub_data.contains_key(&versioned_data.game_spec.game_id) {
            self.checkpoint.root_data.update_sub_data(versioned_data)?;
        } else {
            self.checkpoint.root_data.init_sub_data(versioned_data)?;
        }
        Ok(())
    }

    pub fn print(&self, prefix: String) {
        println!("-------- {} --------", prefix);
        println!("Version: {}", self.settle_version);
        println!("Locks: {:?}", self.settle_locks);
        println!("Settles: {:?}", self.settles);
        println!("Transfer: {:?}", self.transfer);
        println!("Awards: {:?}", self.awards);
        println!("Checkpoint:");
        for data in self.checkpoint.root_data.sub_data.iter() {
            println!("- Id: {}", data.1.game_spec.game_id);
            println!("  Dispatch: {:?}", data.1.dispatch);
            println!("  Bridge Events: {:?}", data.1.bridge_events);
        }
    }
}

/// The context for public data.
///
/// This information is not transmitted over the network, instead it's
/// calculated independently at each node.  This struct will neither
/// be passed into the WASM runtime, instead [`Effect`] will be used.
///
/// # Access Version and Settle Version
///
/// The access version is used to identify the timepoint when a player
/// joined.  If a player has an id that is less equal than current
/// access version in checkpoint, it must be joined before the
/// checkpoint was made. Otherwise, this is a new joined player.
///
/// The settle version is used to identify the transaction version. Every time
/// A transaction is prepared, the settle version will increase by 1.
///
/// # Handler State
///
/// The state of game handler will be serialized as JSON string, and stored.
/// It will be passed into the WASM runtime, and get deseralized inside.
///
/// # Player Exiting
///
/// Players are not always allowed to leave a game.  When leaving,
/// the player will be ejected from the game account, and assets will
/// be paid out.
#[derive(Clone, Debug)]
pub struct GameContext {
    /// The game specification
    pub(crate) game_spec: GameSpec,
    /// Contains `settle_version` and `access_version`
    pub(crate) versions: Versions,
    /// The game status indicating whether the game is running or not. WIP use this variables
    pub(crate) status: GameStatus,
    /// List of nodes serving this game
    pub(crate) nodes: Vec<Node>,
    pub(crate) dispatch: Option<DispatchEvent>,
    pub(crate) handler_state: Vec<u8>,
    /// The balances reported from game bundle
    pub(crate) balances: Vec<PlayerBalance>,
    /// The timestamp for event handling
    pub(crate) timestamp: u64,
    /// All runtime random states, each stores the ciphers and assignments.
    pub(crate) random_states: Vec<RandomState>,
    /// All runtime decision states, each stores the answer.
    pub(crate) decision_states: Vec<DecisionState>,
    /// The current versioned data
    pub(crate) versioned_data: VersionedData,
    /// The SHA256 of current handler state
    pub(crate) state_sha: String,
    /// Accepted deposits, saved for later use in settlement
    pub(crate) accept_deposits: Vec<u64>,
    /// The pending settle details. They are here because the settle is blocked by the settle locks.
    pub(crate) pending_settle_details: Vec<SettleDetails>,
    /// The locks for the under construction settle
    pub(crate) next_settle_locks: HashMap<usize, u64>,
}

impl GameContext {
    pub fn try_new(shared_data: SharedData, versioned_data: VersionedData) -> Result<Self> {
        let SharedData { nodes, balances } = shared_data;

        let versioned_data_1 = versioned_data.clone();

        let VersionedData {
            handler_state, game_spec, versions, ..
        } = versioned_data;

        let state_sha = digest(&handler_state);

        Ok(Self {
            game_spec: game_spec.clone(),
            versions: versions.clone(),
            status: GameStatus::Idle,
            nodes: nodes.clone(),
            balances: balances.clone(),
            dispatch: None,
            timestamp: 0,
            random_states: vec![],
            decision_states: vec![],
            handler_state: handler_state.clone(),
            versioned_data: versioned_data_1,
            state_sha,
            accept_deposits: vec![],
            pending_settle_details: vec![],
            next_settle_locks: HashMap::new(),
        })
    }

    pub fn id_to_addr(&self, id: u64) -> Result<String> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or(Error::CantMapIdToAddr(id))
            .map(|n| n.addr.clone())
    }

    pub fn addr_to_id(&self, addr: &str) -> Result<u64> {
        self.nodes
            .iter()
            .find(|n| n.addr.eq(addr))
            .ok_or(Error::CantMapAddrToId(addr.to_string()))
            .map(|n| n.id)
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn handler_is_initialized(&self) -> bool {
        !self.handler_state.is_empty()
    }

    pub fn get_handler_state_raw(&self) -> &Vec<u8> {
        &self.handler_state
    }

    pub fn set_handler_state_raw(&mut self, state: Vec<u8>) {
        self.handler_state = state;
        self.state_sha = digest(&self.handler_state);
    }

    pub fn get_handler_state<H>(&self) -> H
    where
        H: GameHandler,
    {
        H::try_from_slice(&self.handler_state).unwrap()
    }

    pub fn pending_settle_details_mut(&mut self) -> &mut Vec<SettleDetails> {
        &mut self.pending_settle_details
    }

    pub fn take_first_ready_settle_details(&mut self) -> Option<SettleDetails> {
        // println!("Current pending_settle_details: {:?}", self.pending_settle_details);
        if self
            .pending_settle_details
            .first()
            .is_some_and(|cp| cp.settle_locks.is_empty())
        {
            let cp = self.pending_settle_details.remove(0);
            Some(cp)
        } else {
            None
        }
    }

    pub fn set_handler_state<H>(&mut self, handler: &H)
    where
        H: GameHandler,
    {
        self.set_handler_state_raw(borsh::to_vec(&handler).unwrap())
    }

    pub fn get_nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn get_game_spec(&self) -> &GameSpec {
        &self.game_spec
    }

    pub fn game_addr(&self) -> &str {
        &self.game_spec.game_addr
    }

    pub fn game_id(&self) -> usize {
        self.game_spec.game_id
    }

    // pub fn get_transactor_addr(&self) -> Result<&str> {
    //     self.nodes
    //         .iter()
    //         .find(|n| n.mode == ClientMode::Transactor)
    //         .as_ref()
    //         .map(|n| n.addr.as_str())
    //         .ok_or(Error::InvalidTransactorAddress)
    // }

    pub fn count_nodes(&self) -> u16 {
        self.nodes.len() as u16
    }

    pub fn get_node_by_address(&self, addr: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.addr.eq(addr))
    }

    pub fn get_transactor_node(&self) -> Result<&Node> {
        self.nodes
            .iter()
            .find(|n| n.mode == ClientMode::Transactor)
            .ok_or(Error::CantFindTransactor)
    }

    pub fn dispatch_event(&mut self, event: Event, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(event, self.timestamp + timeout));
    }

    pub fn dispatch_event_instantly(&mut self, event: Event) {
        self.dispatch_event(event, 0);
    }

    pub fn start_game(&mut self) {
        self.dispatch = Some(DispatchEvent::new(
            Event::GameStart,
            0,
        ))
    }

    pub fn wait_timeout(&mut self, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(
            Event::WaitingTimeout,
            self.timestamp + timeout,
        ));
    }

    pub fn action_timeout(&mut self, player_id: u64, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(
            Event::ActionTimeout { player_id },
            self.timestamp + timeout,
        ));
    }

    pub fn shutdown_game(&mut self) {
        self.dispatch = Some(DispatchEvent::new(Event::Shutdown, 0));
    }

    pub fn dispatch_custom<E>(&mut self, e: &E, timeout: u64)
    where
        E: CustomEvent,
    {
        if let Ok(node) = self.get_transactor_node() {
            let event = Event::custom(node.id, e);
            self.dispatch_event(event, timeout);
        }
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_status(&self) -> GameStatus {
        self.status
    }

    pub fn list_random_states(&self) -> &Vec<RandomState> {
        &self.random_states
    }

    pub fn list_random_states_mut(&mut self) -> &mut Vec<RandomState> {
        &mut self.random_states
    }

    pub fn list_decision_states(&self) -> &Vec<DecisionState> {
        &self.decision_states
    }

    pub fn get_dispatch(&self) -> &Option<DispatchEvent> {
        &self.dispatch
    }

    pub fn take_dispatch(&mut self) -> Option<DispatchEvent> {
        let mut dispatch = None;
        std::mem::swap(&mut dispatch, &mut self.dispatch);
        dispatch
    }

    pub fn cancel_dispatch(&mut self) {
        self.dispatch = None;
    }

    pub fn access_version(&self) -> u64 {
        self.versions.access_version
    }

    pub fn settle_version(&self) -> u64 {
        self.versions.settle_version
    }

    /// Get the random state by its id.
    pub fn get_random_state(&self, id: usize) -> Result<&RandomState> {
        if id == 0 {
            return Err(Error::RandomStateNotFound(id));
        }
        if let Some(rnd_st) = self.random_states.get(id - 1) {
            Ok(rnd_st)
        } else {
            Err(Error::RandomStateNotFound(id))
        }
    }

    pub fn get_random_state_unchecked(&self, id: usize) -> &RandomState {
        &self.random_states[id - 1]
    }

    pub fn get_decision_state_mut(&mut self, id: usize) -> Result<&mut DecisionState> {
        if id == 0 {
            return Err(Error::InvalidDecisionId);
        }
        if let Some(st) = self.decision_states.get_mut(id - 1) {
            Ok(st)
        } else {
            Err(Error::InvalidDecisionId)
        }
    }
    /// Get the mutable random state by its id.
    pub fn get_random_state_mut(&mut self, id: usize) -> Result<&mut RandomState> {
        if id == 0 {
            return Err(Error::RandomStateNotFound(id));
        }
        if let Some(rnd_st) = self.random_states.get_mut(id - 1) {
            Ok(rnd_st)
        } else {
            Err(Error::RandomStateNotFound(id))
        }
    }

    /// Assign random item to a player
    pub fn assign(
        &mut self,
        random_id: usize,
        player_addr: String,
        indices: Vec<usize>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.assign(player_addr, indices)?;
        Ok(())
    }

    pub fn reveal(&mut self, random_id: usize, indices: Vec<usize>) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.reveal(indices)?;
        Ok(())
    }

    pub fn release(&mut self, decision_id: usize) -> Result<()> {
        let state = self.get_decision_state_mut(decision_id)?;
        state.release()?;
        Ok(())
    }

    pub fn is_random_ready(&self, random_id: usize) -> bool {
        match self.get_random_state(random_id) {
            Ok(rnd) => matches!(
                rnd.status,
                RandomStatus::Ready | RandomStatus::WaitingSecrets
            ),
            Err(_) => false,
        }
    }

    pub fn is_secrets_ready(&self) -> bool {
        self.random_states
            .iter()
            .all(|st| st.status == RandomStatus::Ready)
    }

    /// Set game status
    pub fn set_game_status(&mut self, status: GameStatus) {
        self.status = status;
    }

    pub fn add_node(&mut self, node_addr: String, access_version: u64, mode: ClientMode) {
        self.nodes.retain(|n| n.addr.ne(&node_addr));
        self.nodes
            .push(Node::new_pending(node_addr, access_version, mode))
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.versions.access_version = access_version;
    }

    pub fn take_accept_deposits(&mut self) -> Vec<u64> {
        take(&mut self.accept_deposits)
    }

    /// Dispatch an event if there's none
    pub fn set_dispatch(&mut self, event: Option<DispatchEvent>) {
        if self.dispatch.is_none() {
            self.dispatch = event;
        }
    }

    /// Dispatch event after timeout.
    pub fn dispatch(&mut self, event: Event, timeout: u64) -> Result<()> {
        if self.dispatch.is_some() {
            return Err(Error::DuplicatedEventDispatching);
        }
        self.dispatch = Some(DispatchEvent::new(event, timeout));
        Ok(())
    }

    pub fn init_random_state(&mut self, spec: RandomSpec) -> Result<usize> {
        let random_id = self.random_states.len() + 1;
        let owners: Vec<String> = self
            .nodes
            .iter()
            .filter_map(|n| {
                if n.status == NodeStatus::Ready
                    && matches!(n.mode, ClientMode::Transactor | ClientMode::Validator)
                {
                    Some(n.addr.clone())
                } else {
                    None
                }
            })
            .collect();

        // The only failure case is that when there are not enough owners.
        // Here we know the game is served, so the servers must not be empty.
        let random_state = RandomState::try_new(random_id, spec, &owners)?;

        self.random_states.push(random_state);
        Ok(random_id)
    }

    pub fn add_shared_secrets(&mut self, _addr: String, shares: Vec<SecretShare>) -> Result<()> {
        for share in shares.into_iter() {
            match share {
                SecretShare::Random {
                    from_addr,
                    to_addr,
                    random_id,
                    index,
                    secret,
                } => {
                    self.get_random_state_mut(random_id)?
                        .add_secret(from_addr, to_addr, index, secret)?;
                }
                SecretShare::Answer {
                    from_addr,
                    decision_id,
                    secret,
                } => {
                    self.get_decision_state_mut(decision_id)?
                        .add_secret(&from_addr, secret)?;
                }
            }
        }
        Ok(())
    }

    pub fn randomize_and_mask(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.mask(addr, ciphertexts)?;
        self.dispatch_randomization_timeout(random_id)
    }

    pub fn lock(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.lock(addr, ciphertexts_and_tests)?;
        self.dispatch_randomization_timeout(random_id)
    }

    pub fn dispatch_randomization_timeout(&mut self, random_id: usize) -> Result<()> {
        let no_dispatch = self.dispatch.is_none();
        let rnd_st = self.get_random_state_mut(random_id)?;
        match rnd_st.status.clone() {
            RandomStatus::Shared => {}
            RandomStatus::Ready => {
                println!("Dispatch RandomnessReady");
                println!("Random State: {:?}", self.random_states.get(random_id));
                self.dispatch_event_instantly(Event::RandomnessReady { random_id });
            }
            RandomStatus::Locking(ref addr) => {
                let id = self.addr_to_id(addr)?;
                if no_dispatch {
                    self.dispatch_event(
                        Event::OperationTimeout { ids: vec![id] },
                        OPERATION_TIMEOUT,
                    );
                }
            }
            RandomStatus::Masking(ref addr) => {
                let id = self.addr_to_id(addr)?;
                if no_dispatch {
                    self.dispatch_event(
                        Event::OperationTimeout { ids: vec![id] },
                        OPERATION_TIMEOUT,
                    );
                }
            }
            RandomStatus::WaitingSecrets => {
                if no_dispatch {
                    let ids = rnd_st
                        .list_operating_addrs()
                        .into_iter()
                        .map(|addr| self.addr_to_id(&addr))
                        .collect::<Result<Vec<u64>>>()?;
                    self.dispatch_event(Event::OperationTimeout { ids }, OPERATION_TIMEOUT);
                }
            }
        }
        Ok(())
    }

    pub fn bump_settle_version(&mut self) -> Result<()> {
        self.versions.settle_version += 1;
        Ok(())
    }

    // Remove all locks with game_id and a version that is smaller than given settle_version
    fn remove_lock_from_pending_settles(&mut self, game_id: usize, settle_version: u64) {
        if let Some(old_version) = self.next_settle_locks.get(&game_id) {
            if *old_version < settle_version {
                self.next_settle_locks.remove(&game_id);
            }
        }
        for settle_detail in self.pending_settle_details.iter_mut() {
            if let Some(old_version) = settle_detail.settle_locks.get(&game_id) {
                if *old_version < settle_version {
                    settle_detail.settle_locks.remove(&game_id);
                }
            }
        }
    }

    pub fn init_sub_game_data(&mut self, versioned_data: VersionedData) -> Result<()> {
        self.versioned_data.init_sub_data(versioned_data)?;
        Ok(())
    }

    pub fn update_sub_game_data(&mut self, versioned_data: VersionedData) -> Result<()> {
        let game_id = versioned_data.game_spec.game_id;
        self.versioned_data.update_sub_data(versioned_data.clone())?;
        self.remove_lock_from_pending_settles(game_id, versioned_data.versions.settle_version);
        Ok(())
    }

    pub fn versioned_data_mut(&mut self) -> &mut VersionedData {
        &mut self.versioned_data
    }

    pub fn versioned_data(&self) -> &VersionedData {
        &self.versioned_data
    }

    pub fn add_revealed_random(
        &mut self,
        random_id: usize,
        revealed: HashMap<usize, String>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st
            .add_revealed(revealed)
            .map_err(|e| Error::InvalidDecryptedValue(e.to_string()))
    }

    pub fn add_revealed_answer(&mut self, decision_id: usize, revealed: String) -> Result<()> {
        let st = self.get_decision_state_mut(decision_id)?;
        st.add_released(revealed)
    }

    pub fn ask(&mut self, owner: String) -> usize {
        let id = self.decision_states.len() + 1;
        let st = DecisionState::new(id, owner);
        self.decision_states.push(st);
        id
    }

    pub fn answer_decision(
        &mut self,
        id: usize,
        owner: &str,
        ciphertext: Ciphertext,
        digest: SecretDigest,
    ) -> Result<()> {
        let st = self.get_decision_state_mut(id)?;
        st.answer(owner, ciphertext, digest)
    }

    pub fn get_revealed(&self, random_id: usize) -> Result<&HashMap<usize, String>> {
        let rnd_st = self.get_random_state(random_id)?;
        Ok(&rnd_st.revealed)
    }

    pub fn derive_effect(&self) -> Effect {
        let revealed = self
            .list_random_states()
            .iter()
            .map(|st| (st.id, st.revealed.clone()))
            .collect();
        let answered = self
            .list_decision_states()
            .iter()
            .filter_map(|st| st.get_revealed().map(|a| (st.id, a.to_owned())))
            .collect();

        let curr_sub_game_id = self.versioned_data.sub_data.len() + 1;

        Effect {
            start_game: false,
            stop_game: false,
            cancel_dispatch: false,
            action_timeout: None,
            wait_timeout: None,
            timestamp: self.timestamp,
            curr_random_id: self.list_random_states().len() + 1,
            curr_decision_id: self.list_decision_states().len() + 1,
            nodes_count: self.count_nodes(),
            asks: Vec::new(),
            assigns: Vec::new(),
            reveals: Vec::new(),
            releases: Vec::new(),
            init_random_states: Vec::new(),
            revealed,
            answered,
            is_checkpoint: false,
            withdraws: Vec::new(),
            ejects: Vec::new(),
            handler_state: Some(self.handler_state.clone()),
            error: None,
            transfer: None,
            launch_sub_games: Vec::new(),
            bridge_events: Vec::new(),
            is_init: false,
            entry_lock: None,
            logs: Vec::new(),
            awards: Vec::new(),
            reject_deposits: Vec::new(),
            accept_deposits: Vec::new(),
            curr_sub_game_id,
            balances: Vec::new(),
        }
    }

    pub fn checkpoint(&self) -> ContextCheckpoint {
        let shared_data = SharedData::new(self.balances.clone(), self.nodes.clone());
        ContextCheckpoint::new(shared_data, self.versioned_data.clone())
    }

    pub fn apply_effect(&mut self, effect: Effect) -> Result<EventEffects> {
        let Effect {
            action_timeout,
            wait_timeout,
            start_game,
            stop_game,
            cancel_dispatch,
            asks,
            assigns,
            reveals,
            releases,
            init_random_states,
            withdraws,
            ejects,
            transfer,
            handler_state,
            is_checkpoint,
            launch_sub_games,
            bridge_events,
            error,
            is_init,
            awards,
            reject_deposits,
            mut accept_deposits,
            balances,
            entry_lock,
            ..
        } = effect;

        // Handle dispatching
        if start_game {
            self.start_game();
        } else if stop_game {
            self.shutdown_game();
        } else if let Some(t) = action_timeout {
            self.action_timeout(t.player_id, t.timeout);
        } else if let Some(t) = wait_timeout {
            self.wait_timeout(t);
        } else if cancel_dispatch {
            self.cancel_dispatch();
        }

        self.accept_deposits.append(&mut accept_deposits);
        self.accept_deposits.sort();
        self.accept_deposits.dedup();

        for Assign {
            random_id,
            indices,
            player_id,
        } in assigns.into_iter()
        {
            let addr = self.id_to_addr(player_id)?;
            self.assign(random_id, addr, indices)?;
        }

        for Reveal { random_id, indices } in reveals.into_iter() {
            self.reveal(random_id, indices)?;
        }

        for Release { decision_id } in releases.into_iter() {
            self.release(decision_id)?;
        }

        for Ask { player_id } in asks.into_iter() {
            let addr = self.id_to_addr(player_id)?;
            self.ask(addr);
        }

        for spec in init_random_states.into_iter() {
            self.init_random_state(spec)?;
        }

        let previous_settle_version = self.settle_version();

        if let Some(state) = handler_state {
            self.set_handler_state_raw(state.clone());
            let mut settles = vec![];

            if is_init {
                self.bump_settle_version()?;
                self.versioned_data = VersionedData::new(self.game_spec.clone(), self.versions, state);
                self.set_game_status(GameStatus::Idle);
            } else if is_checkpoint {
                let settles_map = build_settles_map(&withdraws, &ejects, &self.balances, &balances);
                self.balances = balances;
                settles = settles_map.into_values().collect();
                settles.sort_by_key(|s| s.player_id);

                self.random_states.clear();
                self.decision_states.clear();
                self.bump_settle_version()?;
                self.versioned_data.set_state_and_bump_version(state);
                self.set_game_status(GameStatus::Idle);
            }

            let dispatch = self.dispatch.clone();
            // XXX, we save only handler state in checkpoint
            // we don't save init_account.
            // so when we create a subgame, we first need to get the initial checkpoint under the master game's context
            // then deliver the checkpoint to the subgame for intialization.
            //
            // Append SubGame to context
            //
            // for sub_game in launch_sub_games.iter().cloned() {
            //     self.sub_games.push(sub_game.clone());
            //     // Why
            //     // self.checkpoint_mut().append_launch_subgames(sub_game);
            // }
            self.versioned_data.set_dispatch(dispatch);
            self.versioned_data.set_bridge_events(bridge_events.clone());

            let mut checkpoint: Option<ContextCheckpoint> = None;

            if is_checkpoint || is_init {

                println!("is_checkpoint = {}, is_init = {}", is_checkpoint, is_init);

                let shared_data = SharedData::new(self.balances.clone(), self.nodes.clone());
                let cp = ContextCheckpoint::new(shared_data, self.versioned_data.clone());

                let settle_details = SettleDetails {
                    settles,
                    transfer,
                    awards,
                    checkpoint: cp.clone(),
                    access_version: self.access_version(),
                    settle_version: self.settle_version(),
                    previous_settle_version,
                    entry_lock,
                    accept_deposits: self.accept_deposits.drain(..).collect(),
                    settle_locks: self.next_settle_locks.clone(),
                };
                self.pending_settle_details.push(settle_details);
                checkpoint = Some(cp);
            }

            if self.game_spec.game_id == 0 {
                // Add lock into next_settle_locks
                for evt in bridge_events.clone() {
                    let settle_version = self.versioned_data
                        .sub_data
                        .get(&evt.dest)
                        .map(|d| d.versions.settle_version)
                        .unwrap_or(0);
                    self.next_settle_locks.insert(evt.dest, settle_version);
                }
            }

            return Ok(EventEffects {
                launch_sub_games,
                bridge_events,
                start_game,
                stop_game,
                logs: effect.logs,
                reject_deposits,
                checkpoint,
            });
        } else if let Some(e) = error {
            return Err(Error::HandleError(e));
        } else {
            return Err(Error::InternalError(
                "Missing both state and error".to_string(),
            ));
        }
    }

    pub fn set_node_ready(&mut self, access_version: u64) {
        for n in self.nodes.iter_mut() {
            if let NodeStatus::Pending(a) = n.status {
                if a <= access_version {
                    n.status = NodeStatus::Ready
                }
            }
        }
    }

    pub fn max_players(&self) -> u16 {
        self.game_spec.max_players
    }

    pub fn entry_type(&self) -> &EntryType {
        &self.game_spec.entry_type
    }

    pub fn state_sha(&self) -> String {
        self.state_sha.to_string()
    }

    pub fn versions(&self) -> Versions {
        self.versions
    }

    pub fn get_balances(&self) -> &[PlayerBalance] {
        &self.balances
    }
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            game_spec: Default::default(),
            versions: Default::default(),
            status: GameStatus::Idle,
            nodes: Vec::new(),
            balances: Vec::new(),
            versioned_data: VersionedData::default(),
            dispatch: None,
            handler_state: "".into(),
            timestamp: 0,
            random_states: Vec::new(),
            decision_states: Vec::new(),
            state_sha: "".into(),
            accept_deposits: vec![],
            pending_settle_details: vec![],
            next_settle_locks: HashMap::new(),
        }
    }
}

fn build_settles_map(
    withdraws: &[Withdraw],
    ejects: &[u64],
    old_balances: &[PlayerBalance],
    new_balances: &[PlayerBalance],
) -> HashMap<u64, Settle> {
    // Build settles
    // Settle is a combination of Withdraw, Balance Diff, and Eject
    let mut settles_map: HashMap<u64, Settle> = HashMap::new();

    for withdraw in withdraws {
        settles_map
            .entry(withdraw.player_id)
            .and_modify(|e| e.withdraw += withdraw.amount)
            .or_insert_with(|| Settle::new(withdraw.player_id, withdraw.amount, None, false));
    }

    for eject in ejects.iter() {
        settles_map
            .entry(*eject)
            .and_modify(|e| e.eject = true)
            .or_insert_with(|| Settle::new(*eject, 0, None, true));
    }

    let mut balances_change: HashMap<u64, i128> = old_balances
        .iter()
        .map(|orig_balance| (orig_balance.player_id, -(orig_balance.balance as i128)))
        .collect();

    for balance in new_balances.iter() {
        balances_change
            .entry(balance.player_id)
            .and_modify(|e| *e += balance.balance as i128)
            .or_insert(balance.balance as i128);
    }

    for (player_id, chg) in balances_change {
        let change = match chg {
            _ if chg > 0 => Some(BalanceChange::Add(chg as u64)),
            _ if chg < 0 => Some(BalanceChange::Sub(-chg as u64)),
            _ => None,
        };
        if change.is_some() {
            settles_map
                .entry(player_id)
                .and_modify(|e| e.change = change)
                .or_insert_with(|| Settle::new(player_id, 0, change, false));
        }
    }

    return settles_map;
}

#[cfg(test)]
mod tests {

    // use super::*;
    // use crate::checkpoint::Checkpoint;
    // use race_api::effect::{EmitBridgeEvent, LaunchSubGame, Withdraw};
    // use race_api::init_account::InitAccount;

    // #[test]
    // fn given_effect_with_bridge_event_do_apply_effect() -> anyhow::Result<()> {
    //     let mut ef = Effect::default();
    //     let shared_data = SharedData::new(
    //         vec![],
    //         vec![],
    //     );
    //     let root_data = VersionedData::new(
    //         GameSpec::default(),
    //         Versions::default(),
    //         vec![],
    //     );
    //     let mut ctx = GameContext::try_new(shared_data, root_data)?;

    //     ef.handler_state = Some(vec![1]);
    //     ef.bridge_events = vec![EmitBridgeEvent::new_empty(1)];
    //     ef.is_checkpoint = true;

    //     ctx.apply_effect(ef)?;
    //     assert_eq!(ctx.next_settle_locks.len(), 1);
    //     assert_eq!(ctx.pending_settle_details.len(), 1);
    //     assert_eq!(
    //         ctx.versioned_data().bridge_events.len(),
    //         1
    //     );

    //     // A future call on handle_versioned_data should remove the lock for SettleDetails
    //     let mut vd = VersionedData::default();
    //     vd.game_spec.game_id = 1;
    //     vd.versions.settle_version = 1;

    //     ctx.versioned_data_mut().init_sub_data(vd);
    //     assert!(ctx.pending_settle_details[0]
    //         .checkpoint
    //         .root_data
    //         .sub_data
    //         .get(&1)
    //         .unwrap()
    //         .bridge_events
    //         .is_empty());
    //     assert_eq!(
    //         ctx.pending_settle_details[0].settle_locks,
    //         HashMap::default()
    //     );
    //     Ok(())
    // }

    // #[test]
    // fn given_init_sub_checkpoint_do_handle_versioned_data() {}

    // #[test]
    // fn given_sub_checkpoint_after_main_checkpoint_do_handle_versioned_data() {
    //     let mut ctx = GameContext::default();
    //     let mut vd = VersionedData::default();
    //     let mut cp = Checkpoint::default();
    //     let mut sd = SettleDetails::default();
    //     sd.settle_locks.insert(1, 0);

    //     cp.data.insert(0, VersionedData::default());
    //     cp.data.insert(1, VersionedData::default());
    //     sd.checkpoint = cp.clone();
    //     vd.id = 1;
    //     vd.versions.settle_version = 1;
    //     ctx.pending_settle_details_mut().push(sd);
    //     ctx.checkpoint = cp;

    //     ctx.handle_versioned_data(1, vd.clone(), false).unwrap();
    //     assert_eq!(ctx.next_settle_locks.get(&1), None);
    //     assert_eq!(ctx.next_settle_locks.len(), 0);
    //     assert_eq!(ctx.pending_settle_details[0].settle_locks.len(), 0);
    // }

    // #[test]
    // fn test_apply_effect_with_all_fields() {
    //     // Setting up a GameContext
    //     let mut game_context = GameContext::default();
    //     game_context.versions = Versions::new(10, 10);
    //     game_context.checkpoint =
    //         Checkpoint::new(0, GameSpec::default(), game_context.versions(), vec![1]);
    //     game_context.add_node("server".into(), 0, ClientMode::Transactor);
    //     game_context.add_node("alice".into(), 1, ClientMode::Player);
    //     game_context.add_node("bob".into(), 2, ClientMode::Player);
    //     game_context.set_node_ready(10);

    //     // Defining handler state
    //     let handler_state = vec![1, 2, 3, 4, 5];

    //     // Creating Effect with non-empty fields
    //     let effect = Effect {
    //         start_game: true,
    //         stop_game: false,
    //         cancel_dispatch: true, // cancel_dispatch won't work here, as we also set start_game
    //         action_timeout: None,
    //         wait_timeout: Some(1000),
    //         timestamp: 1234567890,
    //         curr_random_id: 1,
    //         curr_decision_id: 1,
    //         nodes_count: 2,
    //         asks: vec![],
    //         assigns: vec![],
    //         reveals: vec![],
    //         releases: vec![],
    //         init_random_states: vec![RandomSpec::deck_of_cards()],
    //         revealed: HashMap::new(),
    //         answered: HashMap::new(),
    //         is_checkpoint: true,
    //         withdraws: vec![Withdraw {
    //             player_id: 1,
    //             amount: 1000,
    //         }],
    //         ejects: vec![1],
    //         handler_state: Some(handler_state.clone()),
    //         error: None,
    //         transfer: Some(Transfer { amount: 200 }),
    //         launch_sub_games: vec![LaunchSubGame {
    //             id: 1,
    //             bundle_addr: String::from("address"),
    //             init_account: InitAccount {
    //                 max_players: 4,
    //                 data: vec![],
    //             },
    //         }],
    //         bridge_events: vec![EmitBridgeEvent {
    //             dest: 1,
    //             raw: vec![1],
    //         }],
    //         is_init: false,
    //         entry_lock: Some(EntryLock::Closed),
    //         logs: vec![],
    //         awards: vec![Award::new(1, String::from("bonus"))],
    //         reject_deposits: vec![1, 2, 3],
    //         accept_deposits: vec![4, 5, 6],
    //         curr_sub_game_id: 1,
    //         balances: vec![PlayerBalance::new(1, 1000), PlayerBalance::new(2, 2000)],
    //     };

    //     let effect_bridge_events = effect.bridge_events.clone();

    //     // Apply the effect
    //     let event_effects = game_context.apply_effect(effect).unwrap();

    //     assert_eq!(event_effects.launch_sub_games.len(), 1);
    //     assert_eq!(event_effects.bridge_events.len(), 1);
    //     assert!(event_effects.start_game);
    //     assert!(event_effects.checkpoint.is_some());
    //     assert_eq!(event_effects.reject_deposits, vec![1, 2, 3]);
    //     assert_eq!(game_context.settle_version(), 11);
    //     assert_eq!(game_context.pending_settle_details.len(), 1);
    //     let settle_details = game_context.pending_settle_details[0].clone();
    //     assert_eq!(
    //         settle_details.settles,
    //         [
    //             Settle {
    //                 player_id: 1,
    //                 withdraw: 1000,
    //                 change: Some(BalanceChange::Add(1000)),
    //                 eject: true
    //             },
    //             Settle {
    //                 player_id: 2,
    //                 withdraw: 0,
    //                 change: Some(BalanceChange::Add(2000)),
    //                 eject: false
    //             },
    //         ]
    //     );
    //     assert_eq!(settle_details.transfer, Some(Transfer { amount: 200 }));
    //     assert_eq!(settle_details.awards, vec![Award::new(1, "bonus".into())]);
    //     assert!(!settle_details.checkpoint.root_data.sub_data.is_empty());
    //     assert_eq!(settle_details.access_version, 10);
    //     assert_eq!(settle_details.settle_version, 11);
    //     assert_eq!(settle_details.previous_settle_version, 10);
    //     assert_eq!(settle_details.entry_lock, Some(EntryLock::Closed));
    //     assert_eq!(settle_details.accept_deposits, vec![4, 5, 6]);
    //     assert_eq!(settle_details.settle_locks, HashMap::default());
    //     assert_eq!(game_context.next_settle_locks, HashMap::from([(1, 0)]));

    //     let game_id = game_context.game_id();
    //     let bridge_events = game_context
    //         .versioned_data()
    //         .bridge_events
    //         .clone();
    //     assert_eq!(bridge_events, effect_bridge_events);
    //     assert_eq!(game_context.dispatch, Some(DispatchEvent { timeout: 0, event: Event::GameStart }));
    // }

    // // #[test]
    // // fn test_build_settle_map() {
    // //     let withdraws = vec![
    // //         Withdraw::new(1, 100000)
    // //     ];
    // //     let ejects = vec![1];
    // //     let old_balances = vec![];
    // //     let new_balances = vec![PlayerBalance::new(2, 100000)];
    // //     let settles_map = build_settles_map(&withdraws, &ejects, &old_balances, &new_balances);
    // //     assert_eq!(settles_map,
    // //         HashMap::from([(
    // //             1, Settle::new(1, 100000, Some(BalanceChange::Add(100000)), true)
    // //         ), (
    // //             2, Settle::new(2, 0, Some(BalanceChange::Add(100000)), false)
    // //         )]));
    // // }
}
