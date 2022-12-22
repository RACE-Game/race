use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
// HoldemAccount offers necessary data (serialized in vec) to GameAccount for Holdem games
// In the simplest form, it represnets a `table' players can join
#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct HoldemAccount {
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
}

// Table info (may be merged with other stucts)
#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct Table {
    pub nft: String,            // table NFT addr
    pub name: String,           // table name
    pub btn: u8,                // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8,               // table size: total number of players
    pub mode: String,           // game type: cash, sng or tourney?
    pub token: String,          // token should be a struct of its own?
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct Pots {
   pub owner_ids: Vec<String>,
   pub winner_ids: Vec<String>,
   pub amount: u64,
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Debug)]
pub enum Street {
    Init,
    Preflop,
    Flop,
    Turn,
    River,
}

// Players type that contains Bet, PlayerStatus
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
pub struct BetMap {
    pub street_bet: u64,
    pub bet_map: HashMap<String, u64>,
    // consider using a Vec<bet> where bet's index maps to player's position at a table
}

// need a custome game event type
#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameEvent {
    Bet(u64),
    Check,
    Call,
    Fold,
    Raise(u64),
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_betmap() {
        let mut my_map = HashMap::new();
        my_map.insert("abcd1".into(), 200);
        my_map.insert("abcd2".into(), 200);
        my_map.insert("abcd3".into(), 200);
        my_map.insert("abcd4".into(), 200);

        let betmap = BetMap {
            street_bet: 200,
            bet_map: my_map,
        };

        let ser = betmap.try_to_vec().unwrap();
        let ser_d = BetMap::try_from_slice(&ser).unwrap();

        assert_eq!(ser_d, betmap);
    }

}
