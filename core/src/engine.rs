use serde::{de::DeserializeOwned, Serialize};

use crate::{
    context::{GameContext, GameStatus, PlayerStatus},
    error::Result,
    event::Event,
    types::{GameAccount, Settle},
};

pub trait GameHandler: Sized + Serialize + DeserializeOwned {
    /// Initialize handler state with on-chain game account data.
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self>;

    /// Handle event.
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()>;
}

pub fn general_init_state(_context: &mut GameContext, _init_account: &GameAccount) -> Result<()> {
    Ok(())
}

/// A general function for system events handling.
pub fn general_handle_event(context: &mut GameContext, event: &Event) -> Result<()> {
    // Remove current event disptaching
    context.dispatch = None;

    // General event handling
    match event {
        Event::Ready { sender } => context.set_player_status(sender, PlayerStatus::Ready),

        Event::ShareSecrets {
            sender,
            secrets: shares,
        } => {
            context.add_shared_secrets(sender, shares.clone())?;
            if context.secrets_ready() {
                context.disptach(Event::SecretsReady, 0)?;
                context.set_game_status(GameStatus::Running);
            }
            Ok(())
        }

        Event::Mask {
            sender,
            random_id,
            ciphertexts,
        } => context.randomize(sender, *random_id, ciphertexts.clone()),

        Event::Lock {
            sender,
            random_id,
            ciphertexts_and_digests: ciphertexts_and_tests,
        } => context.lock(sender, *random_id, ciphertexts_and_tests.clone()),

        Event::RandomnessReady => Ok(()),

        Event::Join {
            player_addr,
            balance,
            position,
        } => context.add_player(player_addr, *balance, *position),

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
pub fn after_handle_event(old_context: &GameContext, new_context: &mut GameContext) -> Result<()> {
    // Find all leaving player, submit during the settlement.
    // Or create a settlement for just player leaving.
    let mut left_players = vec![];
    for p in old_context.players.iter() {
        if new_context.get_player_by_address(&p.addr).is_none() {
            left_players.push(p.addr.to_owned());
        }
    }

    for p in left_players.into_iter() {
        new_context.add_settle(Settle::eject(p));
    }

    Ok(())
}
