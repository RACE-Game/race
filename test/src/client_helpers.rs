use std::collections::HashMap;

use race_core::{
    context::GameContext,
    error::{Error, Result},
    event::{CustomEvent, Event, SecretIdent},
    random::{RandomMode, RandomStatus},
    types::{ClientMode, GameAccount, SecretKey},
};
use race_crypto::SecretState;
use rand::seq::SliceRandom;

use crate::TEST_TRANSACTOR_ACCOUNT_ADDR;

pub struct TestClient {
    pub decryption: HashMap<(usize, usize), String>,
    pub mode: ClientMode,
    pub transactor_addr: String,
    pub server_addr: String,
    pub secret_states: Vec<SecretState>,
}

impl TestClient {
    pub fn new(mode: ClientMode, init_account: &GameAccount) -> Self {
        let transactor_addr = init_account
            .transactor_addr
            .as_ref()
            .expect("Game is not served")
            .clone();

        Self {
            decryption: HashMap::new(),
            mode,
            transactor_addr,
            server_addr: TEST_TRANSACTOR_ACCOUNT_ADDR.into(),
            secret_states: vec![],
        }
    }

    fn update_secret_state(&mut self, game_context: &GameContext) -> Result<()> {
        let random_states = game_context.list_random_states();
        let secret_states = &mut self.secret_states;
        if random_states.len() > secret_states.len() {
            for random_state in random_states.iter().skip(secret_states.len()) {
                let secret_state =
                    SecretState::from_random_state(random_state, RandomMode::Shuffler);
                secret_states.push(secret_state);
            }
        }
        Ok(())
    }

    fn randomize_and_share(&mut self, game_context: &GameContext) -> Result<Vec<Event>> {
        let mut events = vec![];
        for random_state in game_context.list_random_states().iter() {
            match random_state.status {
                RandomStatus::Ready => (),
                RandomStatus::WaitingSecrets => {
                    // check if our secret is required
                    let required_idents =
                        random_state.list_required_secrets_by_from_addr(&self.server_addr);

                    let shares = required_idents
                        .into_iter()
                        .map(|idt| {
                            if let Some(secret_state) = self.secret_states.get(idt.random_id) {
                                let secret = secret_state.get_key(idt.index)?;
                                Ok((idt, secret))
                            } else {
                                Err(Error::MissingSecret)
                            }
                        })
                        .collect::<Result<HashMap<SecretIdent, SecretKey>>>()?;
                    let event = Event::ShareSecrets {
                        sender: self.server_addr.clone(),
                        secrets: shares,
                    };
                    events.push(event);
                }
                RandomStatus::Locking(ref addr) => {
                    // check if our operation is being requested
                    if self.server_addr.eq(addr) {
                        let secret_state = self
                            .secret_states
                            .get_mut(random_state.id)
                            .expect("Failed to get secret state");

                        let origin = random_state
                            .ciphertexts
                            .iter()
                            .map(|c| c.ciphertext().to_owned())
                            .collect();

                        let unmasked = secret_state
                            .unmask(origin)
                            .map_err(|e| Error::RandomizationError(e.to_string()))?;

                        let locked = secret_state
                            .lock(unmasked)
                            .map_err(|e| Error::RandomizationError(e.to_string()))?;

                        let event = Event::Lock {
                            sender: self.server_addr.clone(),
                            random_id: random_state.id,
                            ciphertexts_and_digests: locked,
                        };

                        events.push(event);
                    }
                }
                RandomStatus::Masking(ref addr) => {
                    // check if our operation is being requested
                    if self.server_addr.eq(addr) {
                        let secret_state = self
                            .secret_states
                            .get_mut(random_state.id)
                            .expect("Failed to get secret state");

                        let origin = random_state
                            .ciphertexts
                            .iter()
                            .map(|c| c.ciphertext().to_owned())
                            .collect();

                        let mut masked = secret_state
                            .mask(origin)
                            .map_err(|e| Error::RandomizationError(e.to_string()))?;

                        let mut rng = rand::thread_rng();
                        masked.shuffle(&mut rng);

                        let event = Event::Mask {
                            sender: self.server_addr.clone(),
                            random_id: random_state.id,
                            ciphertexts: masked,
                        };

                        events.push(event);
                    }
                }
            }
        }

        Ok(events)
    }

    pub fn handle_updated_context(&mut self, ctx: &GameContext) -> Result<Vec<Event>> {
        self.update_secret_state(ctx)?;
        let events = self.randomize_and_share(ctx)?;

        Ok(events)
    }
}

pub struct TestPlayerClient {
    player_addr: String,
}

impl TestPlayerClient {
    pub fn new<S: Into<String>>(player_addr: S) -> Self {
        Self {
            player_addr: player_addr.into(),
        }
    }

    /// Decrypt the ciphertexts with shared secrets.
    /// Return a mapping from mapping from indexes to decrypted value.
    pub fn decrypt(
        &mut self,
        ctx: &GameContext,
        random_id: usize,
    ) -> Result<HashMap<usize, String>> {
        let mut ret = HashMap::new();
        let random_state = ctx.get_random_state(random_id)?;
        let options = &random_state.options;
        let assigned_ciphertexts = random_state.list_assigned_ciphertexts(&self.player_addr);
        let mut shared_secrets = random_state.list_shared_secrets(&self.player_addr)?;
        for (i, mut buf) in assigned_ciphertexts.into_iter() {
            if let Some(secrets) = shared_secrets.remove(&i) {
                race_crypto::apply_multi(secrets, &mut buf);
                let value = String::from_utf8(buf).or(Err(Error::DecryptionFailed))?;
                if !options.contains(&value) {
                    return Err(Error::InvalidDecryptedValue(value))?;
                }
                ret.insert(i, value);
            } else {
                return Err(Error::MissingSecret);
            }
        }
        Ok(ret)
    }

    /// Create a custom event, with signature.
    pub fn create_custom_event<E: CustomEvent>(&self, custom_event: E) -> Event {
        Event::Custom {
            sender: self.player_addr.to_owned(),
            raw: serde_json::to_string(&custom_event).expect("Failed to serialize custom event"),
        }
    }
}
