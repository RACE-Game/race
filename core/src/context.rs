use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;

use crate::engine::GameHandler;
use crate::error::{Error, Result};
use crate::event::CustomEvent;
use crate::random::RandomStatus;
use crate::types::{SecretKey, Settle};
use crate::{
    event::{Event, SecretIdent},
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

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub enum ServerStatus {
    #[default]
    Absent,
    Ready,
    DropOff,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone)]
pub enum GameStatus {
    #[default]
    Uninit,
    Initializing, // initalizing randomness
    Waiting,
    Running,
    Sharing,
    Closed,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct Player {
    pub addr: String,
    pub position: usize,
    pub status: PlayerStatus,
    pub balance: u64,
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

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct Server {
    pub addr: String,
    pub status: ServerStatus,
}

impl Server {
    pub fn new<S: Into<String>>(addr: S) -> Self {
        Server {
            addr: addr.into(),
            status: ServerStatus::Absent,
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
    // Current transactor's address
    pub(crate) transactor_addr: String,
    pub(crate) status: GameStatus,
    /// List of players playing in this game
    pub(crate) players: Vec<Player>,
    /// List of validators serving this game
    pub(crate) servers: Vec<Server>,
    pub(crate) dispatch: Option<DispatchEvent>,
    pub(crate) state_json: String,
    pub(crate) timestamp: u64,
    // Whether a player can leave or not
    pub(crate) allow_leave: bool,
    // All runtime random state, each stores the ciphers and assignments.
    pub(crate) random_states: Vec<RandomState>,
    // Settles, if is not None, will be handled by event loop.
    pub(crate) settles: Option<Vec<Settle>>,
    // /// The encrption keys from every nodes.
    // /// Keys are node address.
    // pub encrypt_keys: HashMap<&'a str, Vec<u8>>,

    // /// The verification keys from every nodes.
    // /// Both players and validators have their verify keys.
    // /// Keys are node address.
    // pub verify_keys: HashMap<&'a str, String>,
}

impl GameContext {
    pub fn new(game_account: &GameAccount) -> Result<Self> {
        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        Ok(Self {
            game_addr: game_account.addr.clone(),
            transactor_addr: transactor_addr.to_owned(),
            status: GameStatus::Uninit,
            players: Default::default(),
            servers: game_account.server_addrs.iter().map(Server::new).collect(),
            dispatch: None,
            state_json: "".into(),
            timestamp: 0,
            allow_leave: false,
            random_states: vec![],
            settles: None,
        })
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn get_handler_state_json(&self) -> &str {
        &self.state_json
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

    pub fn dispatch(&mut self, event: Event, timeout: u64) {
        self.dispatch = Some(DispatchEvent::new(event, timeout));
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

    pub fn get_status(&self) -> GameStatus {
        self.status
    }

    pub(crate) fn set_players(&mut self, players: Vec<Player>) {
        self.players = players;
    }

    pub fn list_random_states(&self) -> &Vec<RandomState> {
        &self.random_states
    }

    pub fn get_dispatch(&self) -> &Option<DispatchEvent> {
        &self.dispatch
    }

    /// Get the random state by its id.
    pub fn get_random_state(&self, id: usize) -> Result<&RandomState> {
        if let Some(rnd_st) = self.random_states.get(id) {
            Ok(rnd_st)
        } else {
            Err(Error::InvalidRandomId)
        }
    }

    pub fn get_random_state_unchecked(&self, id: usize) -> &RandomState {
        &self.random_states[id]
    }

    /// Get the mutable random state by its id.
    pub fn get_random_state_mut(&mut self, id: usize) -> Result<&mut RandomState> {
        if let Some(rnd_st) = self.random_states.get_mut(id) {
            Ok(rnd_st)
        } else {
            Err(Error::InvalidRandomId)
        }
    }

    /// Assign random item to a player
    pub fn assign(
        &mut self,
        random_id: usize,
        player_addr: String,
        indexes: Vec<usize>,
    ) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.assign(player_addr, indexes)?;
        Ok(())
    }

    pub fn reveal(&mut self, random_id: usize, indexes: Vec<usize>) -> Result<()> {
        let rnd_st = self.get_random_state_mut(random_id)?;
        rnd_st.reveal(indexes)?;
        Ok(())
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
    pub fn set_player_status(&mut self, addr: &str, status: PlayerStatus) -> Result<()> {
        if let Some(p) = self.players.iter_mut().find(|p| p.addr.eq(&addr)) {
            p.status = status;
        } else {
            return Err(Error::InvalidPlayerAddress);
        }
        Ok(())
    }

    /// Add player to the game.
    /// Using in custom event handler is not allowed.
    pub fn add_player(&mut self, addr: &str, balance: u64, position: usize) -> Result<()> {
        if self.get_player_by_address(addr).is_some() {
            return Err(Error::PlayerAlreadyJoined);
        }
        self.players.push(Player::new(addr, balance, position));

        Ok(())
    }

    /// Remove player from the game.
    pub fn remove_player(&mut self, addr: &str) -> Result<()> {
        if self.allow_leave {
            self.players.retain(|p| p.addr.eq(&addr));
            Ok(())
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

    pub fn init_random_state(&mut self, rnd: &dyn RandomSpec) -> usize {
        let random_id = self.random_states.len();
        let owners: Vec<String> = self.servers.iter().map(|v| v.addr.clone()).collect();
        let random_state = RandomState::new(random_id, rnd, &owners);
        self.random_states.push(random_state);
        random_id
    }

    pub fn add_shared_secrets(
        &mut self,
        _addr: &str,
        shares: HashMap<SecretIdent, SecretKey>,
    ) -> Result<()> {
        for (idt, secret) in shares.into_iter() {
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

    pub fn add_revealed(
        &mut self,
        random_id: usize,
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
