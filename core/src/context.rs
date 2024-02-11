use std::collections::HashMap;

use crate::types::{ClientMode, GameAccount, SettleWithAddr, SubGameSpec};
use borsh::{BorshDeserialize, BorshSerialize};
use race_api::decision::DecisionState;
use race_api::effect::{Ask, Assign, Effect, EmitBridgeEvent, LaunchSubGame, Release, Reveal};
use race_api::engine::GameHandler;
use race_api::error::{Error, Result};
use race_api::event::{CustomEvent, Event};
use race_api::prelude::BridgeEvent;
use race_api::random::{RandomSpec, RandomState, RandomStatus};
use race_api::types::{
    Addr, Ciphertext, DecisionId, GameStatus, RandomId, SecretDigest, SecretShare, Settle,
    SettleOp, Transfer,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const OPERATION_TIMEOUT: u64 = 15_000;

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum NodeStatus {
    Pending(u64),
    Confirming,
    Ready,
    Disconnected,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Pending(access_version) => write!(f, "pending[{}]", access_version),
            NodeStatus::Confirming => write!(f, "confirming"),
            NodeStatus::Ready => write!(f, "ready"),
            NodeStatus::Disconnected => write!(f, "disconnected"),
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Node {
    pub addr: String,
    pub id: u64,
    pub mode: ClientMode,
    pub status: NodeStatus,
}

impl Node {
    pub fn new_pending<S: Into<String>>(addr: S, access_version: u64, mode: ClientMode) -> Self {
        Self {
            addr: addr.into(),
            id: access_version,
            mode,
            status: NodeStatus::Pending(access_version),
        }
    }

    pub fn new<S: Into<String>>(addr: S, access_version: u64, mode: ClientMode) -> Self {
        Self {
            addr: addr.into(),
            id: access_version,
            mode,
            status: NodeStatus::Ready,
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DispatchEvent {
    pub timeout: u64,
    pub event: Event,
}

impl DispatchEvent {
    pub fn new(event: Event, timeout: u64) -> Self {
        Self { timeout, event }
    }
}

/// The effects of an event, indicates what actions should be taken
/// after the event handling.
///
/// - checkpoint: to send a settlement.
/// - launch_sub_games: to launch a list of sub games.
/// - bridge_events: to send events to sub games.
/// - start_game: to start game.
#[derive(Debug)]
pub struct EventEffects {
    pub settles: Vec<SettleWithAddr>,
    pub transfers: Vec<Transfer>,
    pub checkpoint: Option<Vec<u8>>,
    pub launch_sub_games: Vec<LaunchSubGame>,
    pub bridge_events: Vec<EmitBridgeEvent>,
    pub start_game: bool,
}

/// The context for public data.
///
/// This information is not transmitted over the network, instead it's
/// calculated independently at each node.  This struct will neither
/// be passed into the WASM runtime, instead [`Effect`] will be used.
///
/// # Access Version and Settle Version
///
/// Version numbers used in synchronization with on-chain data.  Every
/// time a settlement is made, the `settle_version` will increase by
/// 1.
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
/// be paid out.  The property `allow_exit` decides whether leaving is
/// allowed at the moment.  If it's disabled, leaving event will be
/// rejected.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct GameContext {
    pub(crate) game_addr: Addr,
    /// Version numbers for player/server access.  This number will be
    /// increased whenever a new player joins or a server gets attached.
    pub(crate) access_version: u64,
    /// Version number for transactor settlement.  This number will be
    /// increased whenever a transaction is sent.
    pub(crate) settle_version: u64,
    pub(crate) status: GameStatus,
    /// List of nodes serving this game
    pub(crate) nodes: Vec<Node>,
    pub(crate) dispatch: Option<DispatchEvent>,
    pub(crate) handler_state: Vec<u8>,
    pub(crate) timestamp: u64,
    /// Whether a player can leave or not
    pub(crate) allow_exit: bool,
    /// All runtime random states, each stores the ciphers and assignments.
    pub(crate) random_states: Vec<RandomState>,
    /// All runtime decision states, each stores the answer.
    pub(crate) decision_states: Vec<DecisionState>,
    /// Settles, if is not None, will be handled by event loop.
    pub(crate) settles: Option<Vec<Settle>>,
    /// Transfers, if is not None, will be handled by event loop.
    pub(crate) transfers: Option<Vec<Transfer>>,
    /// The latest checkpoint state
    pub(crate) checkpoint: Option<Vec<u8>>,
    /// The sub games to launch
    pub(crate) launch_sub_games: Vec<LaunchSubGame>,
    /// The bridge events to emit
    pub(crate) bridge_events: Vec<EmitBridgeEvent>,
    /// Start a new game
    pub(crate) start_game: bool,
    /// Next settle version to use when we bump. It defaults to
    /// current + 1 for a normal game, and is decided by the parent game
    /// for a sub game.
    pub(crate) next_settle_version: u64,
}

impl GameContext {
    pub fn try_new_with_sub_game_spec(spec: SubGameSpec) -> Result<Self> {
        let SubGameSpec {
            game_addr,
            nodes,
            sub_id,
            ..
        } = spec;

        Ok(Self {
            game_addr: format!("{}:{}", game_addr, sub_id),
            nodes,
            settle_version: spec.settle_version,
            access_version: spec.access_version,
            next_settle_version: spec.settle_version + 1,
            ..Default::default()
        })
    }

    pub fn try_new(game_account: &GameAccount) -> Result<Self> {
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;

        let nodes = game_account
            .servers
            .iter()
            .map(|s| {
                Node::new_pending(
                    s.addr.clone(),
                    s.access_version,
                    if s.addr.eq(transactor_addr) {
                        ClientMode::Transactor
                    } else {
                        ClientMode::Validator
                    },
                )
            })
            .collect();

        Ok(Self {
            game_addr: game_account.addr.clone(),
            access_version: game_account.access_version,
            settle_version: game_account.settle_version,
            status: GameStatus::Idle,
            nodes,
            dispatch: None,
            timestamp: 0,
            allow_exit: false,
            random_states: vec![],
            decision_states: vec![],
            settles: None,
            transfers: None,
            handler_state: "".into(),
            checkpoint: None,
            launch_sub_games: vec![],
            bridge_events: vec![],
            start_game: false,
            next_settle_version: game_account.settle_version + 1,
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

    pub fn is_allow_exit(&self) -> bool {
        self.allow_exit
    }

    pub fn get_handler_state_raw(&self) -> &Vec<u8> {
        &self.handler_state
    }

    pub fn set_handler_state_raw(&mut self, state: Vec<u8>) {
        self.handler_state = state;
    }

    pub fn get_handler_state<H>(&self) -> H
    where
        H: GameHandler,
    {
        H::try_from_slice(&self.handler_state).unwrap()
    }

    pub fn get_checkpoint(&self) -> Option<Vec<u8>> {
        self.checkpoint.clone()
    }

    pub fn set_handler_state<H>(&mut self, handler: &H)
    where
        H: GameHandler,
    {
        self.handler_state = handler.try_to_vec().unwrap()
    }

    pub fn get_nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn get_game_addr(&self) -> &str {
        &self.game_addr
    }

    pub fn get_transactor_addr(&self) -> Result<&str> {
        self.nodes
            .iter()
            .find(|n| n.mode == ClientMode::Transactor)
            .as_ref()
            .map(|n| n.addr.as_str())
            .ok_or(Error::InvalidTransactorAddress)
    }

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

    pub fn start_game(&mut self) {
        self.random_states.clear();
        self.start_game = true;
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

    pub fn is_checkpoint(&self) -> bool {
        self.checkpoint.is_some()
    }

    pub fn get_status(&self) -> GameStatus {
        self.status
    }

    // pub(crate) fn set_players(&mut self, players: Vec<Player>) {
    //     self.players = players;
    // }

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

    pub fn cancel_dispatch(&mut self) {
        self.dispatch = None;
    }

    pub fn get_access_version(&self) -> u64 {
        self.access_version
    }

    pub fn get_settle_version(&self) -> u64 {
        self.settle_version
    }

    pub fn get_next_settle_version(&self) -> u64 {
        self.next_settle_version
    }

    /// Get the random state by its id.
    pub fn get_random_state(&self, id: RandomId) -> Result<&RandomState> {
        if id == 0 {
            return Err(Error::RandomStateNotFound(id));
        }
        if let Some(rnd_st) = self.random_states.get(id - 1) {
            Ok(rnd_st)
        } else {
            Err(Error::RandomStateNotFound(id))
        }
    }

    pub fn get_random_state_unchecked(&self, id: RandomId) -> &RandomState {
        &self.random_states[id - 1]
    }

    pub fn get_decision_state_mut(&mut self, id: DecisionId) -> Result<&mut DecisionState> {
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
    pub fn get_random_state_mut(&mut self, id: RandomId) -> Result<&mut RandomState> {
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
        random_id: RandomId,
        player_addr: String,
        indexes: Vec<usize>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.assign(player_addr, indexes)?;
        Ok(())
    }

    pub fn reveal(&mut self, random_id: RandomId, indexes: Vec<usize>) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.reveal(indexes)?;
        Ok(())
    }

    pub fn release(&mut self, decision_id: DecisionId) -> Result<()> {
        let state = self.get_decision_state_mut(decision_id)?;
        state.release()?;
        Ok(())
    }

    pub fn is_random_ready(&self, random_id: RandomId) -> bool {
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

    /// Set player status by address.
    /// Using it in custom event handler is not allowed.
    pub fn set_node_status(&mut self, addr: &str, status: NodeStatus) -> Result<()> {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.addr.eq(&addr)) {
            n.status = status;
        } else {
            return Err(Error::InvalidPlayerAddress);
        }
        Ok(())
    }

    pub fn add_node(&mut self, node_addr: String, access_version: u64, mode: ClientMode) {
        self.nodes.retain(|n| n.addr.ne(&node_addr));
        self.nodes
            .push(Node::new_pending(node_addr, access_version, mode))
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn set_allow_exit(&mut self, allow_exit: bool) {
        self.allow_exit = allow_exit;
    }

    /// Dispatch an event if there's none
    pub fn dispatch_safe(&mut self, event: Event, timeout: u64) {
        if self.dispatch.is_none() {
            self.dispatch = Some(DispatchEvent::new(event, timeout + self.timestamp));
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

    pub fn init_random_state(&mut self, spec: RandomSpec) -> Result<RandomId> {
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
        random_id: RandomId,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.mask(addr, ciphertexts)?;
        self.dispatch_randomization_timeout(random_id)
    }

    pub fn lock(
        &mut self,
        addr: &str,
        random_id: RandomId,
        ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.lock(addr, ciphertexts_and_tests)?;
        self.dispatch_randomization_timeout(random_id)
    }

    pub fn dispatch_randomization_timeout(&mut self, random_id: RandomId) -> Result<()> {
        let no_dispatch = self.dispatch.is_none();
        let rnd_st = self.get_random_state_mut(random_id)?;
        match rnd_st.status.clone() {
            RandomStatus::Shared => {}
            RandomStatus::Ready => {
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

    pub fn settle(&mut self, settles: Vec<Settle>) {
        self.settles = Some(settles);
    }

    pub fn transfer(&mut self, transfers: Vec<Transfer>) {
        self.transfers = Some(transfers);
    }

    pub fn get_settles(&self) -> &Option<Vec<Settle>> {
        &self.settles
    }

    pub fn get_bridge_events<E: BridgeEvent>(&self) -> Result<Vec<E>> {
        self.bridge_events
            .iter()
            .cloned()
            .map(|e| E::try_from_slice(&e.raw).map_err(|_e| Error::DeserializeError))
            .collect()
    }

    pub fn bump_settle_version(&mut self) -> Result<()> {
        if self.next_settle_version <= self.settle_version {
            return Err(Error::CantBumpSettleVersion);
        }
        self.settle_version = self.next_settle_version;
        self.next_settle_version += 1;
        Ok(())
    }

    pub fn update_next_settle_version(&mut self, next_settle_version: u64) {
        self.next_settle_version = u64::max(next_settle_version, self.settle_version + 1);
    }

    pub fn take_event_effects(&mut self) -> Result<EventEffects> {
        let mut settles = vec![];
        let mut transfers = vec![];

        if self.checkpoint.is_some() {
            if let Some(ss) = self.settles.take() {
                for s in ss {
                    let addr = self.id_to_addr(s.id)?;
                    settles.push(SettleWithAddr { addr, op: s.op });
                }
            }

            settles.sort_by_key(|s| match s.op {
                SettleOp::Add(_) => 0,
                SettleOp::Sub(_) => 1,
                SettleOp::Eject => 2,
                SettleOp::AssignSlot(_) => 3,
            });

            if let Some(mut t) = self.transfers.take() {
                transfers.append(&mut t);
            }
            self.bump_settle_version()?;
        }

        let launch_sub_games = self.launch_sub_games.drain(..).collect();

        let bridge_events = self.bridge_events.drain(..).collect();

        Ok(EventEffects {
            settles,
            transfers,
            checkpoint: self.get_checkpoint(),
            launch_sub_games,
            bridge_events,
            start_game: self.start_game,
        })
    }

    pub fn add_settle(&mut self, settle: Settle) {
        if let Some(ref mut settles) = self.settles {
            settles.push(settle);
        } else {
            self.settles = Some(vec![settle]);
        }
    }

    pub fn add_revealed_random(
        &mut self,
        random_id: RandomId,
        revealed: HashMap<usize, String>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st
            .add_revealed(revealed)
            .map_err(|e| Error::InvalidDecryptedValue(e.to_string()))
    }

    pub fn add_revealed_answer(&mut self, decision_id: DecisionId, revealed: String) -> Result<()> {
        let st = self.get_decision_state_mut(decision_id)?;
        st.add_released(revealed)
    }

    pub fn ask(&mut self, owner: String) -> DecisionId {
        let id = self.decision_states.len() + 1;
        let st = DecisionState::new(id, owner);
        self.decision_states.push(st);
        id
    }

    pub fn answer_decision(
        &mut self,
        id: DecisionId,
        owner: &str,
        ciphertext: Ciphertext,
        digest: SecretDigest,
    ) -> Result<()> {
        let st = self.get_decision_state_mut(id)?;
        st.answer(owner, ciphertext, digest)
    }

    pub fn get_revealed(&self, random_id: RandomId) -> Result<&HashMap<usize, String>> {
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
            checkpoint: None,
            settles: Vec::new(),
            handler_state: Some(self.handler_state.clone()),
            error: None,
            allow_exit: self.allow_exit,
            transfers: Vec::new(),
            launch_sub_games: Vec::new(),
            bridge_events: Vec::new(),
        }
    }

    pub fn apply_effect(&mut self, effect: Effect) -> Result<()> {
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
            settles,
            transfers,
            handler_state,
            allow_exit,
            checkpoint,
            launch_sub_games,
            bridge_events,
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

        self.set_allow_exit(allow_exit);

        for Assign {
            random_id,
            indexes,
            player_id,
        } in assigns.into_iter()
        {
            let addr = self.id_to_addr(player_id)?;
            self.assign(random_id, addr, indexes)?;
        }

        for Reveal { random_id, indexes } in reveals.into_iter() {
            self.reveal(random_id, indexes)?;
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

        if let Some(checkpoint_state) = checkpoint {
            self.checkpoint = Some(checkpoint_state);
            self.settle(settles);
            self.transfer(transfers);
            self.set_game_status(GameStatus::Idle);
        } else if (!settles.is_empty()) || (!transfers.is_empty()) {
            return Err(Error::SettleWithoutCheckpoint);
        }

        if let Some(state) = handler_state {
            self.handler_state = state;
        }

        self.launch_sub_games = launch_sub_games;

        self.bridge_events = bridge_events;

        Ok(())
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

    pub fn apply_checkpoint(&mut self, access_version: u64, settle_version: u64) -> Result<()> {
        if self.settle_version != settle_version {
            return Err(Error::InvalidCheckpoint);
        }

        self.access_version = access_version;

        Ok(())
    }

    pub fn prepare_for_next_event(&mut self, timestamp: u64) {
        self.set_timestamp(timestamp);
        self.checkpoint = None;
        self.start_game = false;
        self.bridge_events.clear();
    }
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            game_addr: "".into(),
            access_version: 0,
            settle_version: 0,
            status: GameStatus::Idle,
            nodes: Vec::new(),
            dispatch: None,
            handler_state: "".into(),
            timestamp: 0,
            allow_exit: false,
            random_states: Vec::new(),
            decision_states: Vec::new(),
            settles: None,
            transfers: None,
            checkpoint: None,
            launch_sub_games: Vec::new(),
            bridge_events: Vec::new(),
            start_game: false,
            next_settle_version: 0,
        }
    }
}
