use borsh::{BorshDeserialize, BorshSerialize};
use race_core::event::CustomEvent;
use serde::{Deserialize, Serialize};
// use race_core::event::Event;

// HoldemAccount offers necessary (static) data (serialized in vec) to GameAccount for Holdem games
// This HoldemAccount data go to the (Raw) data field and
// Holdem (the WASM), the actual game, go to the bundle_addr field
#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct HoldemAccount {
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    pub btn: u8,                // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8,               // table size: total number of players
    pub mode: String,           // game type: cash, sng or tourney?
    pub token: String,          // token should be a struct of its own?
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum PlayerStatus {
    #[default]
    Wait,                       // or Idle?
    Acted,
    Acting,                     // or In_Action?
    Allin,
    Fold,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Player {             // or <'p>
    pub addr: String,
    pub chips: u64,
    pub bet: u64,               // current bet
    pub position: u8,           // zero indexed
    pub status: PlayerStatus,   // or &'p str?
    // pub online_status
    // pub drop_count
    // pub timebank
    // pub nickname
}

impl Player {
    pub fn new(id: String, bb: u64, pos: u8) -> Player {
        Self {
            addr: id,
            chips: 20 * bb,     // suppose initial chips are 20 bbs
            bet: 0,
            position: pos,
            status: Default::default(),
        }
    }

    // Whether need to act
    pub fn to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Wait | PlayerStatus::Acted => true,
            _ => false
        }
    }


    pub fn to_remain(&self) -> bool {
        match self.status {
            PlayerStatus::Fold => false,
            _ => true
        }
    }

    pub fn to_take_action(&self) -> bool {
        match self.status {
            PlayerStatus::Allin | PlayerStatus::Fold => false,
            _ => true
        }
    }



    pub fn take_bet(&mut self, bet: u64) {
        // ignore allin for now
        self.chips = self.chips - bet;
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Pot {
   pub owners: Vec<String>,
   pub winners: Vec<String>,
   pub amount: u64,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub enum Street {
    Init,
    Preflop,
    Flop,
    Turn,
    River,
    Done,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Bet {
    owner: String,
    amount: u64,
}

impl Bet {
    pub fn new<S: Into<String>>(owner: S, amount: u64) -> Self {
        Self {
            owner: owner.into(),
            amount
        }
    }

    pub fn get_bet_amount(&self) -> u64 {
        self.amount
    }
}

// Game status for holdem
#[derive(Serialize, Deserialize, Clone)]
pub enum GameStatus {
    Init,
    Encrypt,
    ShareKey,
    Play,
    Runner,
    Settle,
    Showdown,
    Shuffle,

}

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Bet(u64),
    Check,
    Call,
    Fold,
    Raise(u64),
}

impl CustomEvent for GameEvent {}


#[cfg(test)]
mod tests {

    use super::*;
    // #[test]
    // fn test_betmap() {
    //     let mut my_map = HashMap::new();
    //     my_map.insert("abcd1".into(), 200);
    //     my_map.insert("abcd2".into(), 200);
    //     my_map.insert("abcd3".into(), 200);
    //     my_map.insert("abcd4".into(), 200);
    //
    //     let betmap = BetMap {
    //         street_bet: 200,
    //         bet_map: my_map,
    //     };
    //
    //     let ser = betmap.try_to_vec().unwrap();
    //     let ser_d = BetMap::try_from_slice(&ser).unwrap();
    //
    //     assert_eq!(ser_d, betmap);
    // }

}
