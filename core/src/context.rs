use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;

use crate::engine::GameHandler;
use crate::error::{Error, Result};
use crate::event::CustomEvent;
use crate::random::RandomStatus;
use crate::types::{PlayerJoin, RandomId, SecretShare, ServerJoin, Settle, SettleOp};
use crate::{
    event::Event,
    random::{RandomSpec, RandomState},
    types::{Ciphertext, GameAccount},
};

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Serialize)]
pub enum PlayerStatus {
    #[default]
    Absent,
    Ready,
    Disconnected,
    DropOff,
}

#[derive(Debug, Default, Serialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub enum ServerStatus {
    #[default]
    Absent,
    Ready,
    DropOff,
}

#[derive(
    Debug, Default, Serialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone,
)]
pub enum GameStatus {
    #[default]
    Uninit,
    Running,
    Closed,
}

#[derive(Debug, Serialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct Player {
    pub addr: String,
    pub position: usize,
    pub status: PlayerStatus,
    pub balance: u64,
}

impl From<PlayerJoin> for Player {
    fn from(new_player: PlayerJoin) -> Self {
        Self {
            addr: new_player.addr,
            position: new_player.position,
            status: PlayerStatus::Ready,
            balance: new_player.balance,
        }
    }
}

impl Player {
    pub fn new<S: Into<String>>(addr: S, balance: u64, position: usize) -> Self {
        Self {
            addr: addr.into(),
            status: PlayerStatus::Ready,
            balance,
            position,
        }
    }
}

#[derive(Debug, Serialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct Server {
    pub addr: String,
    pub status: ServerStatus,
    pub endpoint: String,
}

impl From<ServerJoin> for Server {
    fn from(new_server: ServerJoin) -> Self {
        Self {
            addr: new_server.addr,
            status: ServerStatus::Ready,
            endpoint: new_server.endpoint,
        }
    }
}

impl Server {
    pub fn new<S: Into<String>>(addr: S, endpoint: String) -> Self {
        Server {
            addr: addr.into(),
            endpoint,
            status: ServerStatus::Ready,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
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
#[derive(Default, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct GameContext {
    pub(crate) game_addr: String,
    /// Version numbers for player/server access.  This number will be
    /// increased whenever a new player joined or a server attached.
    pub(crate) access_version: u64,
    /// Version number for transactor settlement.  This number will be
    /// increased whenever a transaction is sent.
    pub(crate) settle_version: u64,
    /// Current transactor's address
    pub(crate) transactor_addr: String,
    pub(crate) status: GameStatus,
    /// List of players playing in this game
    pub(crate) players: Vec<Player>,
    /// List of validators serving this game
    pub(crate) servers: Vec<Server>,
    /// List of players those paid in the contract, but haven't been
    /// collected by transactor.
    pub(crate) pending_players: Vec<PlayerJoin>,
    /// List of servers those attached in the contract, but haven't been
    /// collected by transactor.
    pub(crate) pending_servers: Vec<ServerJoin>,
    pub(crate) dispatch: Option<DispatchEvent>,
    pub(crate) state_json: String,
    pub(crate) timestamp: u64,
    // Whether a player can leave or not
    pub(crate) allow_exit: bool,
    // All runtime random state, each stores the ciphers and assignments.
    pub(crate) random_states: Vec<RandomState>,
    // Settles, if is not None, will be handled by event loop.
    pub(crate) settles: Option<Vec<Settle>>,
    pub(crate) error: Option<Error>,
}

impl GameContext {
    pub fn try_new(game_account: &GameAccount) -> Result<Self> {
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;

        Ok(Self {
            game_addr: game_account.addr.clone(),
            access_version: game_account.access_version,
            settle_version: game_account.settle_version,
            transactor_addr: transactor_addr.to_owned(),
            status: GameStatus::Uninit,
            players: vec![],
            servers: vec![],
            pending_players: game_account.players.clone(),
            pending_servers: game_account.servers.clone(),
            dispatch: None,
            state_json: "".into(),
            timestamp: 0,
            allow_exit: false,
            random_states: vec![],
            settles: None,
            error: None,
        })
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn get_handler_state_json(&self) -> &str {
        &self.state_json
    }

    pub fn is_allow_exit(&self) -> bool {
        self.allow_exit
    }

    pub fn get_handler_state<H>(&self) -> H
    where
        H: GameHandler,
    {
        serde_json::from_str(&self.state_json).unwrap()
    }

    pub fn set_handler_state<H>(&mut self, handler: &H)
    where
        H: Serialize,
    {
        self.state_json = serde_json::to_string(&handler).unwrap();
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

    pub fn count_players(&self) -> usize {
        self.players.len() + self.pending_players.len()
    }

    pub fn gen_first_event(&self) -> Event {
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

    pub fn dispatch(&mut self, event: Event, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(event, timeout));
    }

    pub fn start_game(&mut self) {
        self.dispatch = Some(DispatchEvent::new(self.gen_first_event(), 0));
    }

    pub fn dispatch_custom<E>(&mut self, e: &E, timeout: u64)
    where
        E: CustomEvent,
    {
        let event = Event::Custom {
            sender: self.transactor_addr.to_owned(),
            raw: serde_json::to_string(e).unwrap(),
        };
        self.dispatch(event, timeout);
    }

    pub fn get_players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn get_pending_players(&self) -> &Vec<PlayerJoin> {
        &self.pending_players
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
        println!("reavel: {:?}", rnd_st);
        rnd_st.reveal(indexes)?;
        Ok(())
    }

    pub fn is_random_ready(&self, random_id: RandomId) -> bool {
        match self.get_random_state(random_id) {
            Ok(rnd) => matches!(
                rnd.status,
                RandomStatus::Ready | RandomStatus::WaitingSecrets(_)
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

    pub fn set_error(&mut self, error: Error) {
        self.error = Some(error)
    }

    pub fn get_error(&self) -> &Option<Error> {
        &self.error
    }

    /// Set game status
    pub fn set_game_status(&mut self, status: GameStatus) {
        self.status = status;
    }

    /// Set player status by address.
    /// Using in custom event handler is not allowed.
    pub fn set_player_status(&mut self, addr: &str, status: PlayerStatus) -> Result<()> {
        if let Some(p) = self.players.iter_mut().find(|p| p.addr.eq(&addr)) {
            p.status = status;
        } else {
            return Err(Error::InvalidPlayerAddress);
        }
        Ok(())
    }

    pub fn add_player(&mut self, index: usize) -> Result<()> {
        let new_player = self.pending_players.remove(index);
        if new_player.access_version > self.access_version {
            self.access_version = new_player.access_version;
        }
        self.players.push(new_player.into());
        Ok(())
    }

    pub fn add_pending_player(&mut self, new_player: PlayerJoin) -> Result<()> {
        if self
            .pending_players
            .iter()
            .find(|p| p.addr.eq(&new_player.addr))
            .is_some()
        {
            return Err(Error::PlayerAlreadyJoined(new_player.addr));
        }
        self.pending_players.push(new_player);
        Ok(())
    }

    /// Add server to the game.
    /// Using in custom event handler is not allowed.
    pub fn add_server(&mut self, index: usize) -> Result<()> {
        let new_server = self.pending_servers.remove(index);
        if new_server.access_version > self.access_version {
            self.access_version = new_server.access_version;
        }
        self.servers.push(new_server.into());
        Ok(())
    }

    pub fn add_pending_server(&mut self, new_server: ServerJoin) -> Result<()> {
        if self
            .pending_servers
            .iter()
            .find(|p| p.addr.eq(&new_server.addr))
            .is_some()
        {
            return Err(Error::ServerAlreadyJoined(new_server.addr));
        }
        self.pending_servers.push(new_server);
        Ok(())
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

    /// Dispatch event after timeout.
    pub fn disptach(&mut self, event: Event, timeout: u64) -> Result<()> {
        if self.dispatch.is_some() {
            return Err(Error::DuplicatedEventDispatching);
        }
        self.dispatch = Some(DispatchEvent::new(event, timeout));
        Ok(())
    }

    pub fn init_random_state(&mut self, rnd: &dyn RandomSpec) -> Result<RandomId> {
        let random_id = self.random_states.len() + 1;
        let owners: Vec<String> = self.servers.iter().map(|v| v.addr.clone()).collect();

        // The only failure case is no enough owners.
        // Here we know the game is served, so the servers must not be empty.
        let random_state = RandomState::try_new(random_id, rnd, &owners)?;

        self.random_states.push(random_state);
        Ok(random_id)
    }

    pub fn add_shared_secrets(&mut self, _addr: &str, shares: Vec<SecretShare>) -> Result<()> {
        for ss in shares.into_iter() {
            let (idt, secret) = ss.into();
            println!("random_id {:?}", idt.random_id);
            let random_state = self.get_random_state_mut(idt.random_id)?;
            random_state.add_secret(idt.from_addr, idt.to_addr, idt.index, secret)?;
        }
        Ok(())
    }

    pub fn randomize(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.mask(addr, ciphertexts)?;

        Ok(())
    }

    pub fn lock(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.lock(addr, ciphertexts_and_tests)?;
        if self.is_all_random_ready() {
            self.dispatch(Event::RandomnessReady, 0);
        }
        Ok(())
    }

    pub fn settle(&mut self, settles: Vec<Settle>) {
        self.settles = Some(settles);
    }

    pub fn get_settles(&self) -> &Option<Vec<Settle>> {
        &self.settles
    }

    pub fn apply_and_take_settles(&mut self) -> Result<Option<Vec<Settle>>> {
        if self.settles.is_some() {
            let mut settles = None;
            std::mem::swap(&mut settles, &mut self.settles);
            for s in settles.as_ref().unwrap().iter() {
                match s.op {
                    SettleOp::Eject => {
                        self.players.retain(|p| p.addr.ne(&s.addr));
                    }
                    SettleOp::Add(amount) => {
                        let p = self
                            .get_player_mut_by_address(&s.addr)
                            .ok_or(Error::InvalidSettle)?;
                        p.balance = p.balance.checked_add(amount).ok_or(Error::InvalidSettle)?;
                    }
                    SettleOp::Sub(amount) => {
                        let p = self
                            .get_player_mut_by_address(&s.addr)
                            .ok_or(Error::InvalidSettle)?;
                        p.balance = p.balance.checked_sub(amount).ok_or(Error::InvalidSettle)?;
                    }
                }
            }
            // Bump the settle version
            // We assume these settlements returned will be proceed
            self.settle_version += 1;
            self.random_states = vec![];
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

    pub fn add_revealed(
        &mut self,
        random_id: RandomId,
        revealed: HashMap<usize, String>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st
            .add_revealed(revealed)
            .map_err(|e| Error::InvalidDecryptedValue(e.to_string()))
    }

    pub fn get_revealed(&self, random_id: usize) -> Result<&HashMap<usize, String>> {
        let rnd_st = self.get_random_state(random_id)?;
        Ok(&rnd_st.revealed)
    }
}
