//! Test serializing and deserializing verious structs used by Holdem
//! as well as the Holdem struct itself

use std::collections::BTreeMap;

use crate::essential::{
    ActingPlayer, AwardPot, Display, GameEvent, HoldemAccount, Player, Pot, ACTION_TIMEOUT_PREFLOP, PlayerResult, PlayerStatus,
};
use crate::game::Holdem;
use crate::tests::helper::{setup_holdem_state, setup_real_holdem};
use borsh::{BorshDeserialize, BorshSerialize};

use super::helper::make_uneven_betmap;

#[test]
fn test_borsh_player() {
    let player = Player::new("Alice".into(), 1000, 1u16);
    let player_ser = player.try_to_vec().unwrap();
    let player_de = Player::try_from_slice(&player_ser).unwrap();
    assert_eq!(player_de, player);
}

#[test]
fn test_borsh_pot() {
    let pot = Pot {
        owners: vec!["Alice".into(), "Bob".into(), "Carol".into()],
        winners: vec!["Alice".into()],
        amount: 120,
    };
    let pot_ser = pot.try_to_vec().unwrap();
    let pot_de = Pot::try_from_slice(&pot_ser).unwrap();
    assert_eq!(pot_de, pot);
}

#[test]
fn test_borsh_holdem_account() {
    let acct = HoldemAccount {
        sb: 10,
        bb: 20,
        rake: 3,
    };
    let acct_ser = acct.try_to_vec().unwrap();
    println!("Game Account Ser data {:?}", acct_ser);
    let acct_de = HoldemAccount::try_from_slice(&acct_ser).unwrap();
    assert_eq!(acct_de, acct);
}

#[test]
fn test_borsh_game_event() {
    let evts = vec![
        GameEvent::Call,
        GameEvent::Bet(20),
        GameEvent::Fold,
        GameEvent::Check,
        GameEvent::Raise(60),
    ];
    for evt in evts.into_iter() {
        println!("Event: {:?}", evt);
        let evt_ser = evt.try_to_vec().unwrap();
        let evt_de = GameEvent::try_from_slice(&evt_ser).unwrap();
        assert_eq!(evt_de, evt);
    }
}

#[test]
fn test_borsh_display() {
    let display = vec![
        Display::DealCards,
        Display::DealBoard {
            prev: 3usize,
            board: vec![
                "sq".to_string(),
                "hq".to_string(),
                "ca".to_string(),
                "dt".to_string(),
                "c6".to_string(),
            ],
        },
        Display::AwardPots {
            pots: vec![
                AwardPot {
                    winners: vec!["Alice".to_string(), "Bob".to_string()],
                    amount: 200,
                },
                AwardPot {
                    winners: vec!["Bob".to_string()],
                    amount: 40,
                },
            ],
        },
        Display::GameResult {
            player_map: BTreeMap::from([
                ("Alice".to_string(),
                    PlayerResult {
                        addr: "Alice".to_string(),
                        position: 0,
                        status: PlayerStatus::Wait,
                        chips: 100,
                        prize: Some(100),
                    }),
                ("Bob".to_string(),
                    PlayerResult {
                        addr: "Bob".to_string(),
                        position: 1,
                        status: PlayerStatus::Out,
                        chips: 0,
                        prize: None,
                })
            ]),
        },
        Display::CollectBets {
            bet_map: make_uneven_betmap(),
        },
    ];

    for dlp in display.into_iter() {
        println!("Display: {:?}", dlp);
        let dlp_ser = dlp.try_to_vec().unwrap();
        let dlp_de = Display::try_from_slice(&dlp_ser).unwrap();
        assert_eq!(dlp_de, dlp);
    }
}

#[test]
fn test_borsh_holdem() {
    let mut holdem = setup_holdem_state().unwrap();
    // Without acting player
    {
        let holdem_ser = holdem.try_to_vec().unwrap();
        println!("== Holdem without an actiing player");
        println!("== Holdem serialized data: {:?}", holdem_ser);
        let holdem_ser_data = [
            1, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 5, 0, 0, 0, 65, 108, 105, 99, 101, 5, 0,
            0, 0, 65, 108, 105, 99, 101, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
            0, 0, 0, 66, 111, 98, 3, 0, 0, 0, 66, 111, 98, 232, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 5, 0, 0, 0, 67, 97, 114, 111, 108, 5, 0, 0, 0, 67, 97, 114, 111, 108,
            232, 3, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 68, 97, 118, 101,
            4, 0, 0, 0, 68, 97, 118, 101, 232, 3, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            3, 0, 0, 0, 69, 118, 97, 3, 0, 0, 0, 69, 118, 97, 232, 3, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 70, 114, 97, 110, 107,
            232, 3, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 3, 0, 0, 0, 66,
            111, 98, 5, 0, 0, 0, 67, 97, 114, 111, 108, 4, 0, 0, 0, 68, 97, 118, 101, 3, 0, 0, 0,
            69, 118, 97, 5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 65, 108, 105, 99, 101, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(holdem_ser, holdem_ser_data);

        let holdem_de = Holdem::try_from_slice(&holdem_ser).unwrap();
        assert_eq!(holdem_de, holdem);
    }

    // With acting player
    {
        let acting_player: Option<ActingPlayer> = Some(ActingPlayer {
            addr: "Alice".into(),
            position: 1usize,
            clock: ACTION_TIMEOUT_PREFLOP,
        });
        holdem.acting_player = acting_player;
        let holdem_ser = holdem.try_to_vec().unwrap();
        println!("== Holdem with an actiing player");
        println!("== Holdem serialized data: {:?}", holdem_ser);
        let holdem_ser_data = vec![1, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 5, 0, 0, 0, 65, 108, 105, 99, 101, 5, 0, 0, 0, 65, 108, 105, 99, 101, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 3, 0, 0, 0, 66, 111, 98, 232, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 67, 97, 114, 111, 108, 5, 0, 0, 0, 67, 97, 114, 111, 108, 232, 3, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 68, 97, 118, 101, 4, 0, 0, 0, 68, 97, 118, 101, 232, 3, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 69, 118, 97, 3, 0, 0, 0, 69, 118, 97, 232, 3, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 70, 114, 97, 110, 107, 232, 3, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 5, 0, 0, 0, 67, 97, 114, 111, 108, 4, 0, 0, 0, 68, 97, 118, 101, 3, 0, 0, 0, 69, 118, 97, 5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 65, 108, 105, 99, 101, 0, 0, 0, 0, 1, 5, 0, 0, 0, 65, 108, 105, 99, 101, 1, 0, 0, 0, 0, 0, 0, 0, 224, 46, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(holdem_ser, holdem_ser_data);
        let holdem_de = Holdem::try_from_slice(&holdem_ser).unwrap();
        assert_eq!(holdem_de, holdem);
    }

    // A real case where 6 table is full and acting player is Bob
    let real_holdem = setup_real_holdem();
    {
        let real_holdem_ser = real_holdem.try_to_vec().unwrap();
        println!("== Real holdem ser data {:?}", real_holdem_ser);

        let real_holdem_de = Holdem::try_from_slice(&real_holdem_ser).unwrap();
        assert_eq!(real_holdem_de, real_holdem);
    }
}
