use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;

use crate::engine::GameHandler;
use crate::error::{Error, Result};
use crate::event::CustomEvent;
use crate::{
    event::{Event, SecretIdent},
    random::{RandomSpec, RandomState},
    types::{Ciphertext, GameAccount},
};

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
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

#[derive(Debug, BorshSerialize, BorshDeserialize, Default, PartialEq, Eq)]
pub enum SecretType {
    #[default]
    Mask,
    Encrypt,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Secret<'a> {
    pub from_addr: &'a str,
    pub to_addr: Option<&'a str>, // None means for public
    pub key: &'a str,
    pub required: bool,
    pub data: String,
    pub secret_type: SecretType,
}

pub struct SecretTest<'a> {
    pub from_addr: &'a str,
    pub to_addr: Option<&'a str>,
    pub test_result: String,
    pub secret_type: SecretType,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
pub struct Player {
    pub addr: String,
    pub status: PlayerStatus,
    pub balance: u64,
}

impl Player {
    pub fn new<S: Into<String>>(addr: S, balance: u64) -> Self {
        Self {
            addr: addr.into(),
            status: PlayerStatus::Ready,
            balance,
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
            status: ServerStatus::Absent
        }
    }
}

pub struct EncryptionKeyContainer {
    pub keys: Vec<String>,
}

#[derive(Default)]
pub enum RandomStatus {
    #[default]
    Init,
    Shuffling,
    Encrypting,
    Ready,
    Broken,
}

/// A structure represents the assignment of a random item. If an
/// item is assigned to a specific player, then every nodes will share
/// their secrets to this player.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct RandomAssign {
    pub random_id: usize,
    pub player_addr: String,
    pub indexes: Vec<usize>,
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
    game_addr: String,
    // Current transactor's address
    transactor_addr: String,
    status: GameStatus,
    /// List of players playing in this game
    players: Vec<Player>,
    /// List of validators serving this game
    servers: Vec<Server>,
    dispatch: Option<DispatchEvent>,
    state_json: String,
    timestamp: u64,
    // Whether a player can leave or not
    allow_leave: bool,
    // All runtime random state, each stores the ciphers and assignments.
    random_states: Vec<RandomState>,
    // Shared secrets
    shared_secrets: HashMap<SecretIdent, String>,
    // /// The encrption keys from every nodes.
    // /// Keys are node address.
    // pub encrypt_keys: HashMap<&'a str, Vec<u8>>,

    // /// The verification keys from every nodes.
    // /// Both players and validators have their verify keys.
    // /// Keys are node address.
    // pub verify_keys: HashMap<&'a str, String>,
}

impl GameContext {
    pub fn new(game_account: &GameAccount) -> Self {
        Self {
            game_addr: game_account.addr.clone(),
            transactor_addr: game_account.transactor_addr.as_ref().unwrap().to_owned(),
            status: GameStatus::Uninit,
            players: Default::default(),
            servers: game_account.server_addrs.iter().map(Server::new).collect(),
            dispatch: None,
            state_json: "".into(),
            timestamp: 0,
            allow_leave: false,
            random_states: vec![],
            shared_secrets: Default::default(),
        }
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

    pub fn game_addr(&self) -> &str {
        &self.game_addr
    }

    pub fn transactor_addr(&self) -> &str {
        &self.transactor_addr
    }

    pub fn get_player_by_index(&self, index: usize) -> Option<&Player> {
        self.players.get(index)
    }

    pub fn get_mut_player_by_index(&mut self, index: usize) -> Option<&mut Player> {
        self.players.get_mut(index)
    }

    pub fn get_player_by_address(&self, addr: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.addr.eq(addr))
    }

    pub fn get_mut_player_by_address(&mut self, addr: &str) -> Option<&mut Player> {
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
        self.dispatch = Some(DispatchEvent::new(event, timeout));
    }

    pub fn players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    pub(crate) fn set_players(&mut self, players: Vec<Player>) {
        self.players = players;
    }

    pub fn list_random_states(&self) -> &Vec<RandomState> {
        &self.random_states
    }

    /// Get the random state by its id.
    pub fn get_random_state(&self, id: usize) -> Result<&RandomState> {
        if let Some(rnd_st) = self.random_states.get(id as usize) {
            Ok(rnd_st)
        } else {
            Err(Error::InvalidRandomId)
        }
    }

    /// Get the mutable random state by its id.
    pub fn get_mut_random_state(&mut self, id: usize) -> Result<&mut RandomState> {
        if let Some(rnd_st) = self.random_states.get_mut(id as usize) {
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
        let rnd_st = self.get_mut_random_state(random_id)?;
        rnd_st.assign(player_addr, indexes)?;
        Ok(())
    }

    /// List all required secrets
    pub fn list_required_secrets(&self) {}

    pub fn list_required_secrets_by_addr(&self, _addr: &str) {}

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
    pub fn add_player(&mut self, addr: &str, balance: u64) -> Result<()> {
        if self.get_player_by_address(addr).is_some() {
            return Err(Error::PlayerAlreadyJoined);
        }
        self.players.push(Player::new(addr, balance));

        Ok(())
    }

    /// Remove player from the game.
    pub fn remove_player(&mut self, addr: &str) -> Result<()> {
        self.players.retain(|p| p.addr.eq(&addr));

        Ok(())
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
        secret_ident: SecretIdent,
        secret_data: String,
    ) -> Result<()> {
        if self.shared_secrets.contains_key(&secret_ident) {
            return Err(Error::DuplicatedSecretSharing);
        }
        self.shared_secrets.insert(secret_ident, secret_data);

        Ok(())
    }

    pub fn randomize(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts: Vec<Ciphertext>,
    ) -> Result<()> {
        let rnd_st = self.get_mut_random_state(random_id)?;
        rnd_st.mask(addr, ciphertexts)?;

        Ok(())
    }

    pub fn lock(
        &mut self,
        addr: &str,
        random_id: usize,
        ciphertexts_and_tests: Vec<(Ciphertext, Ciphertext)>,
    ) -> Result<()> {
        let rnd_st = self.get_mut_random_state(random_id)?;
        rnd_st.lock(addr, ciphertexts_and_tests)?;

        Ok(())
    }

    pub fn settle() {}
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::tests::*;

    #[test]
    fn test_borsh_serialize() {
        let game_account = game_account_with_empty_data();
        let mut ctx = GameContext::new(&game_account);
        ctx.players.push(Player::new("FAKE PLAYER ADDR", 1000));
        let encoded = ctx.try_to_vec().unwrap();
        let decoded = GameContext::try_from_slice(&encoded).unwrap();
        assert_eq!(ctx, decoded);
    }

    #[test]
    fn test_assign() {}
}
