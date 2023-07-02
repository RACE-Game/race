//! Test serializing and deserializing verious structs used by Holdem
//! as well as the Holdem struct itself

use crate::essential::{ActingPlayer, GameEvent, HoldemAccount, Player, Pot};
use crate::game::Holdem;
use crate::tests::helper::{setup_holdem_state, setup_real_holdem};
use borsh::{BorshDeserialize, BorshSerialize};

#[test]
fn test_serde_player() {
    let player = Player::new("Alice".into(), 1000, 1);
    let player_ser = player.try_to_vec().unwrap();
    let player_de = Player::try_from_slice(&player_ser).unwrap();
    assert_eq!(player_de, player);
}

#[test]
fn test_serde_pot() {
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
fn test_serde_holdem_account() {
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
fn test_serde_game_event() {
    let evts = vec![
        GameEvent::Call,
        GameEvent::Bet(20),
        GameEvent::Fold,
        GameEvent::Check,
        GameEvent::Raise(60),
    ];
    for evt in evts.into_iter() {
        let evt_ser = evt.try_to_vec().unwrap();
        let evt_de = GameEvent::try_from_slice(&evt_ser).unwrap();
        assert_eq!(evt_de, evt);
    }
}

#[test]
fn test_serde_holdem() {
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
            0, 0, 65, 108, 105, 99, 101, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
            0, 0, 66, 111, 98, 3, 0, 0, 0, 66, 111, 98, 232, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 5, 0, 0, 0, 67, 97, 114, 111, 108, 5, 0, 0, 0, 67, 97, 114, 111, 108, 232, 3,
            0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 68, 97, 118, 101, 4, 0, 0, 0,
            68, 97, 118, 101, 232, 3, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 69,
            118, 97, 3, 0, 0, 0, 69, 118, 97, 232, 3, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0,
            5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 70, 114, 97, 110, 107, 232, 3, 0, 0, 0,
            0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 5, 0, 0, 0,
            67, 97, 114, 111, 108, 4, 0, 0, 0, 68, 97, 118, 101, 3, 0, 0, 0, 69, 118, 97, 5, 0, 0,
            0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 65, 108, 105, 99, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(holdem_ser, holdem_ser_data);

        let holdem_de = Holdem::try_from_slice(&holdem_ser).unwrap();
        assert_eq!(holdem_de, holdem);
    }

    // With acting player
    {
        let acting_player: Option<ActingPlayer> = Some(("Alice".into(), 1));
        holdem.acting_player = acting_player;
        let holdem_ser = holdem.try_to_vec().unwrap();
        println!("== Holdem with an actiing player");
        println!("== Holdem serialized data: {:?}", holdem_ser);
        let holdem_ser_data = [
            1, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 5, 0, 0, 0, 65, 108, 105, 99, 101, 5, 0,
            0, 0, 65, 108, 105, 99, 101, 232, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
            0, 0, 66, 111, 98, 3, 0, 0, 0, 66, 111, 98, 232, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 5, 0, 0, 0, 67, 97, 114, 111, 108, 5, 0, 0, 0, 67, 97, 114, 111, 108, 232, 3,
            0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 68, 97, 118, 101, 4, 0, 0, 0,
            68, 97, 118, 101, 232, 3, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 69,
            118, 97, 3, 0, 0, 0, 69, 118, 97, 232, 3, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0,
            5, 0, 0, 0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 70, 114, 97, 110, 107, 232, 3, 0, 0, 0,
            0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 5, 0, 0, 0,
            67, 97, 114, 111, 108, 4, 0, 0, 0, 68, 97, 118, 101, 3, 0, 0, 0, 69, 118, 97, 5, 0, 0,
            0, 70, 114, 97, 110, 107, 5, 0, 0, 0, 65, 108, 105, 99, 101, 1, 5, 0, 0, 0, 65, 108,
            105, 99, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
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
