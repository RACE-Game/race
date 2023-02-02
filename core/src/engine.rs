use serde::{de::DeserializeOwned, Serialize};

use crate::{
    context::{GameContext, GameStatus, PlayerStatus},
    encryptor::EncryptorT,
    error::{Error, Result},
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
pub fn general_handle_event(
    context: &mut GameContext,
    event: &Event,
    encryptor: &dyn EncryptorT,
) -> Result<()> {
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

        Event::RandomnessReady { .. } => Ok(()),

        Event::Sync {
            new_players,
            new_servers,
            transactor_addr: _,
            access_version,
        } => {
            if *access_version <= context.access_version {
                return Err(Error::EventIgnored);
            }
            for p in new_players.iter() {
                context.add_pending_player(p.clone())?;
            }
            for s in new_servers.iter() {
                context.add_pending_server(s.clone())?;
            }
            context.access_version = *access_version;

            Ok(())
        }

        Event::Leave { player_addr } => {
            if context
                .players
                .iter()
                .find(|p| p.addr.eq(player_addr))
                .is_none()
            {
                Err(Error::InvalidPlayerAddress)
            } else {
                context.remove_player(player_addr)
            }
        }

        Event::GameStart { access_version } => {
            let mut p_idxs = vec![];
            let mut s_idxs = vec![];
            for (p_idx, p) in context.pending_players.iter().enumerate() {
                if p.access_version <= *access_version {
                    p_idxs.push(p_idx);
                }
            }
            for (s_idx, s) in context.pending_servers.iter().enumerate() {
                if s.access_version <= *access_version {
                    s_idxs.push(s_idx);
                }
            }
            for p_idx in p_idxs.into_iter().rev() {
                context.add_player(p_idx)?;
            }
            for s_idx in s_idxs.into_iter().rev() {
                context.add_server(s_idx)?;
            }
            context.set_game_status(GameStatus::Running);
            Ok(())
        }

        Event::WaitTimeout => Ok(()),

        Event::ActionTimeout { player_addr: _ } => {
            // This event is for game handler
            Ok(())
        }

        Event::SecretsReady => {
            // let decryption = transactor.decrypt(&ctx, 0)?;
            let mut res = vec![];
            for random_state in context.list_random_states() {
                let options = &random_state.options;
                let revealed = encryptor.decrypt_with_secrets(
                    random_state.list_revealed_ciphertexts(),
                    random_state.list_revealed_secrets()?,
                    options,
                )?;
                res.push((random_state.id, revealed));
            }
            for (random_id, revealed) in res.into_iter() {
                context.add_revealed(random_id, revealed)?;
            }
            Ok(())
        }

        _ => Ok(()),
    }
}

/// Context maintaining after event handling.
pub fn post_handle_event(old_context: &GameContext, new_context: &mut GameContext) -> Result<()> {
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
