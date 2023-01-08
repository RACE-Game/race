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
        .map(|p| Player::new(p.addr.to_owned(), 0, p.position))
        .collect();

    context.set_players(players);

    // Accumulate deposits
    for deposit in init_account.deposits.iter() {
        if let Some(p) = context.get_player_mut_by_address(&deposit.addr) {
            p.balance += deposit.amount;
        }
    }

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
pub fn after_handle_event(_context: &mut GameContext) -> Result<()> {
    Ok(())
}
