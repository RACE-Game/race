use borsh::{BorshSerialize, BorshDeserialize};
use serde::Serialize;

use crate::{
    error::{Error, Result},
    event::SecretIdent,
    random::{RandomSpec, RandomState},
};
use std::collections::HashMap;

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum PlayerStatus {
    #[default]
    Absent,
    Ready,
    Disconnected,
    DropOff,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum ValidatorStatus {
    #[default]
    Absent,
    Ready,
    DropOff,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GameStatus {
    #[default]
    Uninit,
    Waiting,
    Running,
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

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Player {
    pub addr: String,
    pub status: PlayerStatus,
}

impl Player {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            status: PlayerStatus::Ready,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Validator {
    pub addr: String,
    pub status: ValidatorStatus,
}

pub struct EncryptionKeyContainer {
    pub keys: Vec<String>,
}

/// The state contains all sensitive information.
/// This state is private.
#[derive(Default)]
pub struct SecretState<'a> {
    /// The secrets used when masking the raw data, each key corresponds to one section.
    mask_secrets: Vec<String>,

    /// The secrets used when encrypting the data, each key corresponds to one item.
    encrypt_secrets: Vec<String>,

    /// Recevied secrets from other nodes.
    recevied_secrets: Vec<Secret<'a>>,
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
pub struct RandomAssign<'a> {
    pub item_id: usize,
    pub player_addr: &'a str,
}

/// The context for secrets holder. This context is for private
/// information, should never be shared with other nodes.
pub struct SecretContext<'a> {
    /// The private key used for sign key share message
    pub sign_key: String,
    /// The private key used to decrypt message
    pub decrypt_key: Vec<u8>,
    /// All runtime states for secret, each item corresponds to a randomness.
    pub secret_states: Vec<SecretState<'a>>,
}

/// The context for public data.
#[derive(Default, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct GameContext {
    pub game_addr: String,
    pub status: GameStatus,
    /// List of players playing in this game
    pub players: Vec<Player>,
    /// List of validators serving this game
    pub transactors: Vec<Validator>,

    // All runtime random state, each stores the ciphers and assignments.
    // pub random_states: Vec<RandomState<'a>>,
    // /// The encrption keys from every nodes.
    // /// Keys are node address.
    // pub encrypt_keys: HashMap<&'a str, Vec<u8>>,

    // /// The verification keys from every nodes.
    // /// Both players and validators have their verify keys.
    // /// Keys are node address.
    // pub verify_keys: HashMap<&'a str, String>,
}

impl GameContext {
    pub fn new<S>(game_addr: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            game_addr: game_addr.into(),
            ..Default::default()
        }
    }

    // /// Initialize the a randomness and return its id in the context.
    // pub fn init_randomness(&mut self, rnd: &dyn RandomSpec) -> usize {
    //     let id = self.random_states.len();
    //     let opts = rnd.options();
    //     // self.random_states.push(Default::default());
    //     id
    // }

    // /// Get the random state by its id.
    // pub fn get_random_state(&self, id: usize) -> Result<&RandomState> {
    //     if let Some(rnd_st) = self.random_states.get(id) {
    //         Ok(rnd_st)
    //     } else {
    //         Err(Error::InvalidRandomId)
    //     }
    // }

    // /// Get the mutable random state by its id.
    // pub fn get_mut_random_state(&'a mut self, id: usize) -> Result<&'a mut RandomState> {
    //     if let Some(rnd_st) = self.random_states.get_mut(id) {
    //         Ok(rnd_st)
    //     } else {
    //         Err(Error::InvalidRandomId)
    //     }
    // }

    // /// Assign a random item to a player.
    // pub fn assign(&'a mut self, random_id: usize, item_id: usize, player_addr: &'a str) -> Result<()> {
    //     Ok(())
    // }

    // /// Reveal a random item to public.
    // pub fn reveal(&'a mut self, random_id: usize, item_id: usize) -> Result<()> {
    //     let rnd_st = self.get_mut_random_state(random_id)?;
    //     if item_id >= rnd_st.ciphertexts.len() {
    //         return Err(Error::InvalidRandomnessRevealing);
    //     }
    //     // rnd_st.reveals.push(item_id);
    //     Ok(())
    // }

    // pub fn submit_mask(&mut self, submitter_addr: &str, random_id: usize, ciphertexts: Vec<String>) {}

    // pub fn submit_unmask() {}

    // /// Commit the random result to context
    // pub fn submit_determined_random(&mut self, submitter_addr: &str, random_id: usize, ciphertexts: Vec<String>) {}

    // /// Commit a branch for future randomness
    // pub fn commit_branch_random(
    //     &mut self,
    //     submitter_addr: &str,
    //     random_id: usize,
    //     key: String,
    //     ciphertexts: Vec<String>,
    // ) {
    // }

    // /// Prepare the random items
    // pub fn prepare(&mut self, random_id: usize, item_ids: Vec<usize>) {}

    // pub fn apply_secret(&mut self, secret_ident: SecretIdent, secret_data: String) {}
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_borsh_serialize() {
        let ctx = GameContext::new("FAKE GAME ADDR");
        let encoded = ctx.try_to_vec().unwrap();
        let decoded = GameContext::try_from_slice(&encoded).unwrap();
        assert_eq!(ctx, decoded);
    }
}
