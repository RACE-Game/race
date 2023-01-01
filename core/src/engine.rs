use serde::{de::DeserializeOwned, Serialize};

use crate::{
    context::{GameContext, GameStatus, Player, PlayerStatus},
    error::Result,
    event::Event,
    types::GameAccount,
};

pub trait GameHandler: Sized + Serialize + DeserializeOwned {
    /// Initialize handler state with on-chain game account data.
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self>;

    /// Handle event.
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()>;
}

pub fn general_init_state(context: &mut GameContext, init_account: &GameAccount) -> Result<()> {
    let players = init_account
        .players
        .iter()
        .map(|p| Player::new(p.addr.to_owned(), p.balance))
        .collect();

    context.set_players(players);

    Ok(())
}

/// A general function for system events handling.
pub fn general_handle_event(context: &mut GameContext, event: &Event) -> Result<()> {
    match event {
        Event::Ready { sender } => context.set_player_status(&sender, PlayerStatus::Ready),

        Event::ShareSecrets {
            sender,
            secret_ident,
            secret_data,
        } => context.add_shared_secrets(sender, secret_ident.clone(), secret_data.clone()),

        Event::Randomize {
            sender,
            random_id,
            ciphertexts,
        } => context.randomize(sender, *random_id, ciphertexts.clone()),

        Event::Lock {
            sender,
            random_id,
            ciphertexts_and_tests,
        } => context.lock(sender, *random_id, ciphertexts_and_tests.clone()),

        Event::RandomnessReady => Ok(()),

        Event::Join {
            player_addr,
            balance,
        } => context.add_player(player_addr, *balance),

        Event::Leave { player_addr } => context.remove_player(player_addr),

        Event::GameStart => {
            context.set_game_status(GameStatus::Initializing);
            Ok(())
        }

        Event::WaitTimeout => Ok(()),

        Event::ActionTimeout { player_addr: _ } => {
            // This event is for game handler
            Ok(())
        }

        Event::SecretsReady => Ok(()),

        _ => Ok(()),
    }
}

/// Context maintaining after event handling.
pub fn after_handle_event(context: &mut GameContext) -> Result<()> {
    // If any secrets are required, set current status to SecretSharing.
    context.set_game_status(GameStatus::Sharing);

    Ok(())
}

#[cfg(test)]
mod tests {
    use borsh::BorshSerialize;

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        counter_value_default: u64,
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData {
            counter_value_default: 42,
        }
        .try_to_vec()
        .unwrap();
        GameAccount {
            addr: "ACC ADDR".into(),
            bundle_addr: "GAME ADDR".into(),
            served: true,
            settle_version: 0,
            access_version: 0,
            players: vec![],
            data_len: data.len() as _,
            data,
            nodes: vec![],
            max_players: 2,
        }
    }

    #[test]
    fn test_general_handle_event_join() {
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        let event = Event::Join {
            player_addr: "Alice".into(),
            balance: 1000,
        };
        general_handle_event(&mut ctx, &event).expect("handle event error");
        assert_eq!(1, ctx.players().len());
    }

    // #[test]
    // fn test_general_handle_event_ready() {
    //     let game_account = make_game_account();
    //     let mut ctx = GameContextAccess::new(GameContext::new(&game_account));
    //     let event = Event::Ready { sender: "Alice".into() };
    //     general_handle_event(&mut ctx, &event).expect_err("handle event should failed");
    //     ctx.players.push(Player::new("Alice", 1000));
    //     general_handle_event(&mut ctx, &event).expect("handle event failed");
    //     assert_eq!(PlayerStatus::Ready, ctx.players[0].status);
    // }

    #[test]
    fn test_general_handle_event_share_secrets() {}

    #[test]
    fn test_general_handle_event_randomize() {}

    #[test]
    fn test_general_handle_event_lock() {}

    #[test]
    fn test_general_handle_event_game_start() {}

    #[test]
    fn test_general_handle_event_draw_random_items() {}
}
