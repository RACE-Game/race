//! Holdem essentials such as bet, player, pot, street and so on.

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::prelude::{CustomEvent, HandleError};
use std::collections::BTreeMap;

#[derive(BorshSerialize, BorshDeserialize, Default, PartialEq, Debug)]
pub enum PlayerStatus {
    #[default]
    Wait,
    Acted,
    Acting,
    Allin,
    Fold,
    Init,            // Indicating new players ready for the next hand
    Winner,
    // Leave,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Player {
    pub addr: String,
    pub chips: u64,
    pub position: usize, // zero indexed
    pub status: PlayerStatus,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr && self.position == other.position
    }
}

impl Player {
    pub fn new(addr: String, chips: u64, position: u16) -> Player {
        Self {
            addr,
            chips,
            position: position as usize,
            status: PlayerStatus::default(),
        }
    }

    pub fn init(addr: String, chips: u64, position: u16) -> Player {
        Self {
            addr,
            chips,
            position: position as usize,
            status: PlayerStatus::Init,
        }
    }

    pub fn next_to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Allin | PlayerStatus::Fold | PlayerStatus::Init => false,
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


#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
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


#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Default)]
pub enum Street {
    #[default]
    Init,
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}


#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
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


// A pot used for awarding winners
#[cfg_attr(test, derive(PartialEq))]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct AwardPot {
    pub winners: Vec<String>,
    pub amount: u64,
}

/// Used for animation (with necessary audio effects)
#[cfg_attr(test, derive(PartialEq))]
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Display {
    DealCards,
    DealBoard { prev: usize, board: Vec<String> },
    CollectBets { bet_map: BTreeMap<String, u64> },
    UpdateChips { player: String, before: u64, after: u64 },
    AwardPots { pots: Vec<AwardPot> },
}

pub const ACTION_TIMEOUT: u64 = 30_000;
pub const WAIT_TIMEOUT: u64 = 10_000;
