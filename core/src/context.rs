use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use borsh::{BorshDeserialize, BorshSerialize};
use crate::decision::DecisionState;
use crate::effect::{Ask, Assign, Effect, Release, Reveal};
use crate::engine::GameHandler;
use crate::error::{Error, Result};
use crate::event::CustomEvent;
use crate::random::{RandomSpec, RandomStatus};
use crate::types::{
    Addr, DecisionId, PlayerJoin, RandomId, SecretDigest, SecretShare, ServerJoin, Settle, SettleOp,
};
use crate::{
    event::Event,
    random::RandomState,
    types::{Ciphertext, GameAccount},
};

const OPERATION_TIMEOUT: u64 = 15_000;

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum NodeStatus {
    Pending(u64),
    Ready,
    Disconnected,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Pending(access_version) => write!(f, "pending[{}]", access_version),
            NodeStatus::Ready => write!(f, "ready"),
            NodeStatus::Disconnected => write!(f, "disconnected"),
        }
    }
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone)]
pub enum GameStatus {
    #[default]
    Uninit,
    Running,
    Closed,
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatus::Uninit => write!(f, "uninit"),
            GameStatus::Running => write!(f, "running"),
            GameStatus::Closed => write!(f, "closed"),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Player {
    pub addr: String,
    pub position: usize,
    pub status: NodeStatus,
    pub balance: u64,
}

impl From<PlayerJoin> for Player {
    fn from(new_player: PlayerJoin) -> Self {
        Self {
            addr: new_player.addr,
            position: new_player.position as _,
            status: NodeStatus::Ready,
            balance: new_player.balance,
        }
    }
}

impl Player {
    pub fn new_pending(addr: String, balance: u64, position: usize, access_version: u64) -> Self {
        Self {
            addr,
            balance,
            position,
            status: NodeStatus::Pending(access_version),
        }
    }

    pub fn new<S: Into<String>>(addr: S, balance: u64, position: usize) -> Self {
        Self {
            addr: addr.into(),
            status: NodeStatus::Ready,
            balance,
            position,
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Server {
    pub addr: String,
    pub status: NodeStatus,
    pub endpoint: String,
}

impl From<ServerJoin> for Server {
    fn from(new_server: ServerJoin) -> Self {
        Self {
            addr: new_server.addr,
            status: NodeStatus::Ready,
            endpoint: new_server.endpoint,
        }
    }
}

impl Server {
    pub fn new_pending<S: Into<String>>(addr: S, endpoint: String, access_version: u64) -> Self {
        Server {
            addr: addr.into(),
            endpoint,
            status: NodeStatus::Pending(access_version),
        }
    }

    pub fn new<S: Into<String>>(addr: S, endpoint: String) -> Self {
        Server {
            addr: addr.into(),
            endpoint,
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
/// Players are not always allowed to leave game.  By leaving game,
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
    /// Current transactor's address
    pub(crate) transactor_addr: Addr,
    pub(crate) status: GameStatus,
    /// List of players playing in this game
    pub(crate) players: Vec<Player>,
    /// List of validators serving this game
    pub(crate) servers: Vec<Server>,
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
}

impl GameContext {
    pub fn try_new(game_account: &GameAccount) -> Result<Self> {
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;

        let players = game_account
            .players
            .iter()
            .map(|p| {
                Player::new_pending(p.addr.clone(), p.balance, p.position as _, p.access_version)
            })
            .collect();

        let servers = game_account
            .servers
            .iter()
            .map(|s| Server::new_pending(s.addr.clone(), s.endpoint.clone(), s.access_version))
            .collect();

        Ok(Self {
            game_addr: game_account.addr.clone(),
            access_version: game_account.access_version,
            settle_version: game_account.settle_version,
            transactor_addr: transactor_addr.to_owned(),
            status: GameStatus::Uninit,
            players,
            servers,
            dispatch: None,
            timestamp: 0,
            allow_exit: false,
            random_states: vec![],
            decision_states: vec![],
            settles: None,
            handler_state: "".into(),
        })
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

    pub fn get_handler_state<H>(&self) -> H
    where
        H: GameHandler,
    {
        H::try_from_slice(&self.handler_state).unwrap()
    }

    pub fn set_handler_state<H>(&mut self, handler: &H)
    where
        H: GameHandler,
    {
        self.handler_state = handler.try_to_vec().unwrap()
    }

    pub fn get_servers(&self) -> &Vec<Server> {
        &self.servers
    }

    pub fn get_game_addr(&self) -> &str {
        &self.game_addr
    }

    pub fn get_transactor_addr(&self) -> &str {
        &self.transactor_addr
    }

    pub fn get_player_by_index(&self, index: usize) -> Option<&Player> {
        self.players.get(index)
    }

    pub fn get_player_mut_by_index(&mut self, index: usize) -> Option<&mut Player> {
        self.players.get_mut(index)
    }

    pub fn get_player_by_address(&self, addr: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.addr.eq(addr))
    }

    pub fn get_player_mut_by_address(&mut self, addr: &str) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.addr.eq(addr))
    }

    pub fn count_players(&self) -> u16 {
        self.players.len() as u16
    }

    pub fn count_servers(&self) -> u16 {
        self.servers.len() as u16
    }

    pub fn gen_start_game_event(&self) -> Event {
        Event::GameStart {
            access_version: self.access_version,
        }
    }

    pub fn get_server_by_address(&self, addr: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.addr.eq(addr))
    }

    pub fn get_transactor_server(&self) -> &Server {
        self.get_server_by_address(&self.transactor_addr).unwrap()
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

    pub fn action_timeout(&mut self, player_addr: String, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(
            Event::ActionTimeout { player_addr },
            self.timestamp + timeout,
        ));
    }

    pub fn start_game(&mut self) {
        self.random_states.clear();
        self.decision_states.clear();
        self.dispatch = Some(DispatchEvent::new(self.gen_start_game_event(), 0));
    }

    pub fn shutdown_game(&mut self) {
        self.dispatch = Some(DispatchEvent::new(Event::Shutdown, 0));
    }

    pub fn dispatch_custom<E>(&mut self, e: &E, timeout: u64)
    where
        E: CustomEvent,
    {
        let event = Event::custom(self.transactor_addr.to_owned(), e);
        self.dispatch_event(event, timeout);
    }

    pub fn get_players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
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

    /// Get the random state by its id.
    pub fn get_random_state(&self, id: RandomId) -> Result<&RandomState> {
        if id == 0 {
            return Err(Error::InvalidRandomId);
        }
        if let Some(rnd_st) = self.random_states.get(id - 1) {
            Ok(rnd_st)
        } else {
            Err(Error::InvalidRandomId)
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
            return Err(Error::InvalidRandomId);
        }
        if let Some(rnd_st) = self.random_states.get_mut(id - 1) {
            Ok(rnd_st)
        } else {
            Err(Error::InvalidRandomId)
        }
    }

    /// Assign random item to a player
    pub fn assign<S: Into<String>>(
        &mut self,
        random_id: RandomId,
        player_addr: S,
        indexes: Vec<usize>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.assign(player_addr.into(), indexes)?;
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

    pub fn is_all_random_ready(&self) -> bool {
        self.random_states
            .iter()
            .all(|st| st.status == RandomStatus::Ready)
    }

    pub fn secrets_ready(&self) -> bool {
        self.random_states
            .iter()
            .all(|st| st.status == RandomStatus::Ready)
    }

    /// Set game status
    pub fn set_game_status(&mut self, status: GameStatus) {
        self.status = status;
    }

    /// Set player status by address.
    /// Using in custom event handler is not allowed.
    pub fn set_player_status(&mut self, addr: &str, status: NodeStatus) -> Result<()> {
        if let Some(p) = self.players.iter_mut().find(|p| p.addr.eq(&addr)) {
            p.status = status;
        } else {
            return Err(Error::InvalidPlayerAddress);
        }
        Ok(())
    }

    /// Add player to the game.
    pub fn add_player(&mut self, player: &PlayerJoin) -> Result<()> {
        if let Some(p) = self
            .players
            .iter()
            .find(|p| p.addr.eq(&player.addr) || p.position == player.position as usize)
        {
            if p.position == player.position as usize {
                Err(Error::PositionOccupied(p.position))
            } else {
                Err(Error::PlayerAlreadyJoined(player.addr.clone()))
            }
        } else {
            self.players.push(Player::new(
                player.addr.clone(),
                player.balance,
                player.position as _,
            ));
            Ok(())
        }
    }

    /// Add server to the game.
    pub fn add_server(&mut self, server: &ServerJoin) -> Result<()> {
        if self
            .servers
            .iter()
            .find(|s| s.addr.eq(&server.addr))
            .is_some()
        {
            Err(Error::ServerAlreadyJoined(server.addr.clone()))
        } else {
            self.servers
                .push(Server::new(server.addr.clone(), server.endpoint.clone()));
            Ok(())
        }
    }

    pub fn set_access_version(&mut self, access_version: u64) {
        self.access_version = access_version;
    }

    pub fn set_allow_exit(&mut self, allow_exit: bool) {
        self.allow_exit = allow_exit;
    }

    /// Remove player from the game.
    pub fn remove_player(&mut self, addr: &str) -> Result<()> {
        let orig_len = self.players.len();
        if self.allow_exit {
            self.players.retain(|p| p.addr.ne(&addr));
            if orig_len == self.players.len() {
                Err(Error::PlayerNotInGame)
            } else {
                Ok(())
            }
        } else {
            Err(Error::CantLeave)
        }
    }

    /// Dispatch an event if there's none
    pub fn dispatch_safe(&mut self, event: Event, timeout: u64) {
        if self.dispatch.is_none() {
            self.dispatch = Some(DispatchEvent::new(event, timeout));
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
            .servers
            .iter()
            .filter_map(|s| {
                if s.status == NodeStatus::Ready {
                    Some(s.addr.clone())
                } else {
                    None
                }
            })
            .collect();

        // The only failure case is no enough owners.
        // Here we know the game is served, so the servers must not be empty.
        let random_state = RandomState::try_new(random_id, spec, &owners)?;

        self.random_states.push(random_state);
        Ok(random_id)
    }

    pub fn add_shared_secrets(&mut self, _addr: &str, shares: Vec<SecretShare>) -> Result<()> {
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

    pub fn dispatch_randomization_timeout(&mut self, random_id: RandomId) -> Result<()> {
        let no_dispatch = self.dispatch.is_none();
        let rnd_st = self.get_random_state_mut(random_id)?;
        match &rnd_st.status {
            RandomStatus::Ready => {
                self.dispatch_event_instantly(Event::RandomnessReady { random_id });
            }
            RandomStatus::Locking(ref addr) => {
                let addr = addr.to_owned();
                if no_dispatch {
                    self.dispatch_event(
                        Event::OperationTimeout { addrs: vec![addr] },
                        OPERATION_TIMEOUT,
                    );
                }
            }
            RandomStatus::Masking(ref addr) => {
                let addr = addr.to_owned();
                if no_dispatch {
                    self.dispatch_event(
                        Event::OperationTimeout { addrs: vec![addr] },
                        OPERATION_TIMEOUT,
                    );
                }
            }
            RandomStatus::WaitingSecrets => {
                if no_dispatch {
                    let addrs = rnd_st.list_operating_addrs();
                    self.dispatch_event(Event::OperationTimeout { addrs }, OPERATION_TIMEOUT);
                }
            }
        }
        Ok(())
    }

    pub fn settle(&mut self, settles: Vec<Settle>) {
        self.settles = Some(settles);
    }

    pub fn get_settles(&self) -> &Option<Vec<Settle>> {
        &self.settles
    }

    pub fn bump_settle_version(&mut self) {
        self.settle_version += 1;
    }

    pub fn apply_and_take_settles(&mut self) -> Result<Option<Vec<Settle>>> {
        if self.settles.is_some() {
            let mut settles = None;
            std::mem::swap(&mut settles, &mut self.settles);

            if let Some(settles) = settles.as_mut() {
                settles.sort_by_key(|s| match s.op {
                    SettleOp::Add(_) => 0,
                    SettleOp::Sub(_) => 1,
                    SettleOp::Eject => 2,
                })
            }

            for s in settles.as_ref().unwrap().iter() {
                match s.op {
                    SettleOp::Eject => {
                        self.players.retain(|p| p.addr.ne(&s.addr));
                    }
                    SettleOp::Add(amount) => {
                        let p =
                            self.get_player_mut_by_address(&s.addr)
                                .ok_or(Error::InvalidSettle(format!(
                                    "Invalid player address: {}",
                                    s.addr
                                )))?;
                        p.balance =
                            p.balance
                                .checked_add(amount)
                                .ok_or(Error::InvalidSettle(format!(
                                    "Settle amount overflow (add): balance {}, change {}",
                                    p.balance, amount,
                                )))?;
                    }
                    SettleOp::Sub(amount) => {
                        let p =
                            self.get_player_mut_by_address(&s.addr)
                                .ok_or(Error::InvalidSettle(format!(
                                    "Invalid player address: {}",
                                    s.addr
                                )))?;
                        p.balance =
                            p.balance
                                .checked_sub(amount)
                                .ok_or(Error::InvalidSettle(format!(
                                    "Settle amount overflow (sub): balance {}, change {}",
                                    p.balance, amount,
                                )))?;
                    }
                }
            }
            self.bump_settle_version();
            Ok(settles)
        } else {
            Ok(None)
        }
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
            handler_state,
            ..
        } = effect;

        // Handle dispatching
        if start_game {
            self.start_game();
        } else if stop_game {
            self.shutdown_game();
        } else if let Some(t) = action_timeout {
            self.action_timeout(t.player_addr, t.timeout);
        } else if let Some(t) = wait_timeout {
            self.wait_timeout(t);
        } else if cancel_dispatch {
            self.cancel_dispatch();
        }

        for Assign {
            random_id,
            indexes,
            player_addr,
        } in assigns.into_iter()
        {
            self.assign(random_id, player_addr, indexes)?;
        }

        for Reveal { random_id, indexes } in reveals.into_iter() {
            self.reveal(random_id, indexes)?;
        }

        for Release { decision_id } in releases.into_iter() {
            self.release(decision_id)?;
        }

        for Ask { player_addr } in asks.into_iter() {
            self.ask(player_addr);
        }

        for spec in init_random_states.into_iter() {
            self.init_random_state(spec)?;
        }

        if !settles.is_empty() {
            self.settle(settles);
        }

        if let Some(state) = handler_state {
            self.handler_state = state;
        }

        Ok(())
    }

    pub fn set_node_ready(&mut self, access_version: u64) {
        for s in self.servers.iter_mut() {
            if let NodeStatus::Pending(a) = s.status {
                if a <= access_version {
                    s.status = NodeStatus::Ready
                }
            }
        }
        for p in self.players.iter_mut() {
            if let NodeStatus::Pending(a) = p.status {
                if a <= access_version {
                    p.status = NodeStatus::Ready
                }
            }
        }
    }

    pub fn apply_checkpoint(&mut self, access_version: u64, settle_version: u64) -> Result<()> {
        if self.settle_version != settle_version {
            return Err(Error::InvalidCheckpoint);
        }

        self.players.retain(|p| match p.status {
            NodeStatus::Pending(v) => v <= access_version,
            NodeStatus::Ready => true,
            NodeStatus::Disconnected => true,
        });

        self.servers.retain(|s| match s.status {
            NodeStatus::Pending(v) => v <= access_version,
            NodeStatus::Ready => true,
            NodeStatus::Disconnected => true,
        });

        self.access_version = access_version;

        Ok(())
    }
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            game_addr: "".into(),
            access_version: 0,
            settle_version: 0,
            transactor_addr: "".into(),
            status: GameStatus::Uninit,
            players: Vec::new(),
            servers: Vec::new(),
            dispatch: None,
            handler_state: "".into(),
            timestamp: 0,
            allow_exit: false,
            random_states: Vec::new(),
            decision_states: Vec::new(),
            settles: None,
        }
    }
}
