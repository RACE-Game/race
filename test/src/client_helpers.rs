use std::collections::HashMap;

use race_core::{
    context::GameContext,
    error::{Result, Error},
    event::{Event, SecretIdent},
    random::{RandomMode, RandomStatus},
    types::{ClientMode, GameAccount},
};
use race_crypto::SecretState;

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
                    let required_idents = random_state
                        .list_required_secrets_by_from_addr(&self.server_addr);
                    let shares = required_idents
                        .into_iter()
                        .map(|idt| {
                            if let Some(secret_state) =
                                self.secret_states.get(idt.random_id)
                            {
                                let secret = secret_state.get_key(idt.index)?;
                                Ok((idt, secret))
                            } else {
                                Err(Error::MissingSecret)
                            }
                        })
                        .collect::<Result<HashMap<SecretIdent, String>>>()?;
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

                        let locked = secret_state
                            .lock(origin)
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

                        let masked = secret_state
                            .mask(origin)
                            .map_err(|e| Error::RandomizationError(e.to_string()))?;

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
