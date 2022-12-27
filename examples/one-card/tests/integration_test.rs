use std::{collections::HashMap, fs::create_dir};

use race_core::{
    context::GameContext,
    error::Result,
    event::Event,
    types::{GameAccount, Player},
};
use race_example_one_card::{GameEvent, OneCard, OneCardGameAccountData};
use race_test::{create_test_game_account, TestHandler};

fn create_handler(ctx: &mut GameContext) -> Result<TestHandler<OneCard>> {
    let acc_data = OneCardGameAccountData {};
    let players = vec![Some(Player::new("Alice", 1000))];
    let init_acc = create_test_game_account(players, 2, acc_data);
    TestHandler::<OneCard>::init_state(ctx, init_acc)
}

#[test]
fn test() -> Result<()> {
    let mut ctx = GameContext::default();
    let mut handler = create_handler(&mut ctx)?;

    {
        assert_eq!(1, ctx.players().len());
        let state: &OneCard = handler.get_state();
        assert_eq!(0, state.dealer);
        assert_eq!(HashMap::from([("Alice".into(), 1000)]), state.chips);
        assert_eq!(HashMap::new(), state.bets);
    }

    let join_event = Event::Join {
        player_addr: "Bob".into(),
        balance: 1000,
    };
    handler.handle_event(&mut ctx, join_event)?;

    {
        let state: &OneCard = handler.get_state();
        assert_eq!(2, ctx.players().len());
        assert_eq!(
            HashMap::from([("Alice".into(), 1000), ("Bob".into(), 1000)]),
            state.chips
        );
    }

    Ok(())
}
