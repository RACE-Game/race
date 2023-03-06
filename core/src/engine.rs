use borsh::{BorshDeserialize, BorshSerialize};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    context::{GameContext, GameStatus, NodeStatus},
    effect::Effect,
    encryptor::EncryptorT,
    error::{Error, Result},
    event::Event,
    prelude::ServerJoin,
    types::{GameAccount, PlayerJoin, Settle},
};

/// A subset of on-chain account, used for game handler
/// initialization.  The `access_version` may refer to an old state
/// when the game is started by transactor.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitAccount {
    pub addr: String,
    pub players: Vec<PlayerJoin>,
    pub servers: Vec<ServerJoin>,
    pub data: Vec<u8>,
    pub access_version: u64,
    pub settle_version: u64,
}

impl InitAccount {
    pub fn from_game_account(
        game_account: GameAccount,
        transactor_access_version: u64,
        transactor_settle_version: u64,
    ) -> Self {
        let players = game_account
            .players
            .into_iter()
            .filter(|p| p.access_version <= transactor_access_version)
            .collect();
        let servers = game_account
            .servers
            .into_iter()
            .filter(|s| s.access_version <= transactor_access_version)
            .collect();

        Self {
            addr: game_account.addr,
            players,
            servers,
            data: game_account.data.clone(),
            access_version: transactor_access_version,
            settle_version: transactor_settle_version,
        }
    }

    pub fn data<S: BorshDeserialize>(&self) -> Result<S> {
        S::try_from_slice(&self.data).or(Err(Error::DeserializeError))
    }
}

impl Default for InitAccount {
    fn default() -> Self {
        Self {
            addr: "".into(),
            players: Vec::new(),
            servers: Vec::new(),
            data: Vec::new(),
            access_version: 0,
            settle_version: 0,
        }
    }
}

pub trait GameHandler: Sized + Serialize + DeserializeOwned {
    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> Result<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<()>;
}

pub fn general_init_state(_context: &mut GameContext, _init_account: &InitAccount) -> Result<()> {
    Ok(())
}

/// A general function for system events handling.
pub fn general_handle_event(
    context: &mut GameContext,
    event: &Event,
    encryptor: &dyn EncryptorT,
) -> Result<()> {
    // General event handling
    match event {
        Event::Ready { sender } => context.set_player_status(sender, NodeStatus::Ready),

        Event::ShareSecrets { sender, shares } => {
            context.add_shared_secrets(sender, shares.clone())?;
            if context.secrets_ready() {
                context.dispatch_event(Event::SecretsReady, 0);
            }
            Ok(())
        }

        Event::Mask {
            sender,
            random_id,
            ciphertexts,
        } => context.randomize_and_mask(sender, *random_id, ciphertexts.clone()),

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
                context.add_player(p)?;
            }
            for s in new_servers.iter() {
                context.add_server(s)?;
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

        Event::GameStart { .. } => {
            context.set_game_status(GameStatus::Running);
            Ok(())
        }

        Event::OperationTimeout { addrs: _ } => {
            // This event is for game handler
            Ok(())
        }

        Event::WaitingTimeout => Ok(()),

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
                context.add_revealed_random(random_id, revealed)?;
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

#[cfg(test)]
mod tests {

    use crate::encryptor::tests::DummyEncryptor;
    use crate::types::ServerJoin;

    use super::*;

    #[test]
    fn test_handle_game_start() -> anyhow::Result<()> {
        let encryptor = DummyEncryptor::default();
        let mut context = GameContext::default();
        let event = Event::GameStart { access_version: 1 };
        general_handle_event(&mut context, &event, &encryptor)?;
        assert_eq!(context.status, GameStatus::Running);
        Ok(())
    }

    #[test]
    fn test_handle_sync() -> anyhow::Result<()> {
        let encryptor = DummyEncryptor::default();
        let mut context = GameContext::default();
        let event = Event::Sync {
            new_players: vec![
                PlayerJoin {
                    addr: "alice".into(),
                    position: 0,
                    balance: 100,
                    access_version: 1,
                },
                PlayerJoin {
                    addr: "bob".into(),
                    position: 1,
                    balance: 100,
                    access_version: 1,
                },
            ],
            new_servers: vec![ServerJoin {
                addr: "foo".into(),
                endpoint: "foo.endpoint".into(),
                access_version: 1,
            }],
            transactor_addr: "".into(),
            access_version: 1,
        };

        general_handle_event(&mut context, &event, &encryptor)?;

        assert_eq!(context.count_players(), 2);
        assert_eq!(context.count_servers(), 1);
        Ok(())
    }
}
