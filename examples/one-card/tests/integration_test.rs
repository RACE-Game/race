use std::{collections::HashMap, fs::create_dir};

use race_core::{
    context::{GameContext, GameStatus},
    error::Result,
    event::Event,
    types::{GameAccount, Player},
};
use race_example_one_card::{GameEvent, OneCard, OneCardGameAccountData};
use race_core_test::{game_account_with_empty_data, TestHandler};

fn create_handler(ctx: &mut GameContext) -> Result<TestHandler<OneCard>> {
    let acc_data = OneCardGameAccountData {};
    let init_acc = game_account_with_empty_data(2, acc_data);
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
        assert_eq!(GameStatus::Initializing, ctx.status());
        assert_eq!(
            HashMap::from([("Alice".into(), 1000), ("Bob".into(), 1000)]),
            state.chips
        );
    }

    Ok(())
}
