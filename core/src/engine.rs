use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    context::{GameContext, GameStatus},
    effect::Effect,
    encryptor::EncryptorT,
    error::{Error, HandleError},
    event::Event,
    prelude::ServerJoin,
    random::RandomStatus,
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
    pub fn from_game_account(game_account: &GameAccount) -> Self {
        let game_account = game_account.to_owned();
        let access_version = game_account.access_version;
        let settle_version = game_account.settle_version;
        let players = game_account.players.clone();
        let servers = game_account.servers.clone();
        Self {
            addr: game_account.addr,
            players,
            servers,
            data: game_account.data.clone(),
            access_version,
            settle_version,
        }
    }

    pub fn new(
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

    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }

    /// Add a new player.  This function is only available in tests.
    /// This function will panic when a duplicated position is
    /// specified.
    pub fn add_player<S: Into<String>>(
        &mut self,
        addr: S,
        position: usize,
        balance: u64,
        verify_key: String,
    ) {
        self.access_version += 1;
        let access_version = self.access_version;
        if self.players.iter().any(|p| p.position as usize == position) {
            panic!("Failed to add player, duplicated position");
        }
        self.players.push(PlayerJoin {
            position: position as _,
            balance,
            addr: addr.into(),
            access_version,
            verify_key,
        })
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

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> Result<Self, HandleError>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError>;
}

pub fn general_init_state(
    _context: &mut GameContext,
    _init_account: &InitAccount,
) -> Result<(), HandleError> {
    Ok(())
}

/// A general function for system events handling.
pub fn general_handle_event(
    context: &mut GameContext,
    event: &Event,
    encryptor: &dyn EncryptorT,
) -> Result<(), Error> {
    // General event handling
    match event {
        Event::Ready => {
            // This is the first event, we make it a checkpoint
            context.checkpoint = true;
            Ok(())
        }

        Event::ShareSecrets { sender, shares } => {
            context.add_shared_secrets(sender, shares.clone())?;
            let mut random_ids = Vec::<usize>::default();
            for random_state in context.list_random_states_mut() {
                if random_state.status == RandomStatus::Shared {
                    random_ids.push(random_state.id);
                    random_state.status = RandomStatus::Ready;
                }
            }
            if !random_ids.is_empty() {
                context.dispatch_event_instantly(Event::SecretsReady { random_ids });
            }
            Ok(())
        }

        Event::AnswerDecision {
            sender,
            decision_id,
            ciphertext,
            digest,
        } => {
            context.answer_decision(*decision_id, sender, ciphertext.clone(), digest.clone())?;
            Ok(())
        }

        Event::Mask {
            sender,
            random_id,
            ciphertexts,
        } => {
            context.randomize_and_mask(sender, *random_id, ciphertexts.clone())?;
            Ok(())
        }

        Event::Lock {
            sender,
            random_id,
            ciphertexts_and_digests: ciphertexts_and_tests,
        } => {
            context.lock(sender, *random_id, ciphertexts_and_tests.clone())?;
            Ok(())
        }

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
                Ok(())
            }
        }

        Event::GameStart { access_version } => {
            context.set_game_status(GameStatus::Running);
            context.set_node_ready(*access_version);
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

        Event::SecretsReady { random_ids } => {
            let mut res = vec![];

            for rid in random_ids {
                if let Ok(random_state) = context.get_random_state_mut(*rid) {
                    let options = &random_state.options;
                    let revealed = encryptor
                        .decrypt_with_secrets(
                            random_state.list_revealed_ciphertexts(),
                            random_state.list_revealed_secrets()?,
                            options,
                        )
                        .or(Err(Error::DecryptionFailed))?;
                    res.push((random_state.id, revealed));
                }
            }
            for (random_id, revealed) in res.into_iter() {
                context.add_revealed_random(random_id, revealed)?;
            }

            let mut res = vec![];
            for decision_state in context.list_decision_states() {
                let secret = decision_state.get_secret()?;
                let mut buf = decision_state
                    .get_answer()
                    .ok_or(Error::InvalidDecisionAnswer)?
                    .ciphertext
                    .clone();
                encryptor.apply(secret, &mut buf);
                res.push((
                    decision_state.id,
                    String::from_utf8(buf).or(Err(Error::DecryptionFailed))?,
                ));
            }
            for (decision_id, revealed) in res.into_iter() {
                context.add_revealed_answer(decision_id, revealed)?;
            }
            Ok(())
        }

        _ => Ok(()),
    }
}

/// Context maintaining after event handling.
pub fn post_handle_event(
    old_context: &GameContext,
    new_context: &mut GameContext,
) -> Result<(), Error> {
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
                    verify_key: "VERIFY KEY".into(),
                },
                PlayerJoin {
                    addr: "bob".into(),
                    position: 1,
                    balance: 100,
                    access_version: 1,
                    verify_key: "VERIFY KEY".into(),
                },
            ],
            new_servers: vec![ServerJoin {
                addr: "foo".into(),
                endpoint: "foo.endpoint".into(),
                access_version: 1,
                verify_key: "VERIFY KEY".into(),
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
