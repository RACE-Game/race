//! Holdem essentials such as bet, player, pot, street and so on.

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::prelude::{CustomEvent, HandleError};

// Bet
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Bet {
    pub owner: String,
    pub amount: u64,
}

impl Bet {
    pub fn new(owner: String, amount: u64) -> Self {
        Self { owner, amount }
    }
}

// Player
#[derive(BorshSerialize, BorshDeserialize, Default, PartialEq, Clone, Debug)]
pub enum PlayerStatus {
    #[default]
    Wait,
    Acted,
    Acting,
    Allin,
    Fold,
    // Leave,
}

#[derive(BorshSerialize, BorshDeserialize, Default, Debug, Clone)]
pub struct Player {
    pub addr: String,
    pub chips: u64,
    pub position: usize, // zero indexed
    pub status: PlayerStatus,
    // pub online_status
    // pub drop_count
    // pub timebank
    // pub nickname
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr && self.position == other.position
    }
}

impl Player {
    pub fn new(addr: String, chips: u64, position: usize) -> Player {
        Self {
            addr,
            chips,
            position,
            status: PlayerStatus::default(),
        }
    }

    pub fn next_to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Allin | PlayerStatus::Fold => false,
            _ => true,
        }
    }

    // This fn indicates
    // 1. whether player goes all in
    // 2. the actual bet amount
    // 3. the player's remaining chips after the bet
    pub fn take_bet(&mut self, bet: u64) -> (bool, u64) {
        if bet < self.chips {
            println!("== Take {} from {}", bet, &self.addr);
            self.chips -= bet;
            (false, bet)
        } else {
            println!("== {} ALL IN: {}", &self.addr, bet);
            let real_bet = self.chips;
            self.chips = 0;
            (true, real_bet)
        }
    }
}

pub type ActingPlayer = (String, usize);


// Pot
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Default, Clone, Debug)]
pub struct Pot {
    pub owners: Vec<String>,
    pub winners: Vec<String>,
    pub amount: u64,
}

impl Pot {
    pub fn new() -> Self {
        Self {
            owners: Vec::<String>::new(),
            winners: Vec::<String>::new(),
            amount: 0,
        }
    }

    pub fn merge(&mut self, other: &Pot) -> Result<(), HandleError> {
        self.amount += other.amount;
        Ok(())
    }
}


// Street
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Default, Clone)]
pub enum Street {
    #[default]
    Init,
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}


// Misc
#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum HoldemStage {
    #[default]
    Init,
    ShareKey,
    Play,
    Runner,
    Settle,
    Showdown,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct HoldemAccount {
    pub sb: u64,
    pub bb: u64,
    pub rake: u16,               // an integer representing the rake percent
}

impl Default for HoldemAccount {
    fn default() -> Self {
        Self {
            sb: 10,
            bb: 20,
            rake: 3u16,
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum GameEvent {
    Bet(u64),
    Check,
    Call,
    Fold,
    Raise(u64),
}

impl CustomEvent for GameEvent {}

pub const ACTION_TIMEOUT: u64 = 30_000;
pub const WAIT_TIMEOUT: u64 = 10_000;
