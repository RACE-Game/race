use std::{collections::HashMap, sync::Arc};

use tracing::info;

use crate::{
    connection::ConnectionT,
    context::GameContext,
    encryptor::EncryptorT,
    error::{Error, Result},
    event::{CustomEvent, Event},
    random::{RandomMode, RandomStatus},
    secret::SecretState,
    transport::TransportT,
    types::{AttachGameParams, Ciphertext, ClientMode, SecretKey, SecretShare, SubmitEventParams},
};

pub struct Client {
    pub encryptor: Arc<dyn EncryptorT>,
    pub transport: Arc<dyn TransportT>,
    pub connection: Arc<dyn ConnectionT>,
    pub game_addr: String,
    // The address of current node, the player address or server address.
    pub addr: String,
    // The client mode, could be player, validator or transactor.
    // Only the player can send custom event.
    // Only the transactor can send system event.
    pub mode: ClientMode,
    // The state of secrets, should match the state of randomness.
    pub secret_states: Vec<SecretState>,
}

impl Client {
    pub fn try_new(
        addr: String,
        game_addr: String,
        mode: ClientMode,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        connection: Arc<dyn ConnectionT>,
    ) -> Result<Self> {
        Ok(Self {
            addr,
            game_addr,
            mode,
            secret_states: Vec::new(),
            transport,
            encryptor,
            connection,
        })
    }

    pub async fn attach_game(&self) -> Result<()> {
        let key = self.encryptor.export_public_key(None)?;
        self.connection
            .attach_game(&self.game_addr, AttachGameParams { key })
            .await
    }

    pub async fn submit_custom_event<S: CustomEvent>(&self, custom_event: S) -> Result<()> {
        let event = Event::custom(&self.game_addr, &custom_event);
        self.connection
            .submit_event(&self.addr, SubmitEventParams { event })
            .await
    }

    fn update_secret_state(&mut self, game_context: &GameContext) -> Result<()> {
        let random_states = game_context.list_random_states();
        let secret_states = &mut self.secret_states;
        if random_states.len() > secret_states.len() {
            for random_state in random_states.iter().skip(secret_states.len()) {
                let secret_state = SecretState::from_random_state(
                    self.encryptor.clone(),
                    random_state,
                    RandomMode::Shuffler,
                );
                info!(
                    "Create secret state for random id: {}, with mode: {:?}",
                    random_state.id,
                    RandomMode::Shuffler
                );
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
                        random_state.list_required_secrets_by_from_addr(&self.addr);

                    let shares = required_idents
                        .into_iter()
                        .map(|idt| {
                            if let Some(secret_state) = self.secret_states.get(idt.random_id) {
                                let secret = secret_state.get_key(idt.index)?;
                                Ok(SecretShare::new(idt, secret))
                            } else {
                                Err(Error::MissingSecret)
                            }
                        })
                        .collect::<Result<Vec<SecretShare>>>()?;
                    let event = Event::ShareSecrets {
                        sender: self.addr.clone(),
                        secrets: shares,
                    };
                    events.push(event);
                }
                RandomStatus::Locking(ref addr) => {
                    // check if our operation is being requested
                    if self.addr.eq(addr) {
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
                            sender: self.addr.clone(),
                            random_id: random_state.id,
                            ciphertexts_and_digests: locked,
                        };

                        events.push(event);
                    }
                }
                RandomStatus::Masking(ref addr) => {
                    // check if our operation is being requested
                    if self.addr.eq(addr) {
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

                        self.encryptor.shuffle(&mut masked);

                        let event = Event::Mask {
                            sender: self.addr.clone(),
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

    pub async fn handle_updated_context(&mut self, ctx: &GameContext) -> Result<()> {
        info!("Client handle updated context in mode: {:?}", self.mode);
        let events = match self.mode {
            ClientMode::Player => {
                self.update_secret_state(ctx)?;
                vec![]
            }
            ClientMode::Transactor | ClientMode::Validator => {
                self.update_secret_state(ctx)?;
                self.randomize_and_share(ctx)?
            }
        };
        for event in events.into_iter() {
            self.connection
                .submit_event(&self.game_addr, SubmitEventParams { event })
                .await?;
        }
        Ok(())
    }

    /// Decrypt the ciphertexts with shared secrets.
    /// Return a mapping from mapping from indexes to decrypted value.
    pub fn decrypt(
        &mut self,
        ctx: &GameContext,
        random_id: usize,
    ) -> Result<HashMap<usize, String>> {
        let random_state = ctx.get_random_state(random_id)?;
        let options = &random_state.options;

        let mut revealed = decrypt_with_secrets(
            &*self.encryptor,
            random_state.list_revealed_ciphertexts(),
            random_state.list_revealed_secrets()?,
            options,
        )?;

        if self.mode == ClientMode::Player {
            let assigned = decrypt_with_secrets(
                &*self.encryptor,
                random_state.list_assigned_ciphertexts(&self.addr),
                random_state.list_shared_secrets(&self.addr)?,
                options,
            )?;
            revealed.extend(assigned);
        }

        Ok(revealed)
    }
}

fn decrypt_with_secrets(
    encryptor: &dyn EncryptorT,
    ciphertext_map: HashMap<usize, Ciphertext>,
    mut secret_map: HashMap<usize, Vec<SecretKey>>,
    options: &[String],
) -> Result<HashMap<usize, String>> {
    let mut ret = HashMap::new();
    for (i, mut buf) in ciphertext_map.into_iter() {
        if let Some(secrets) = secret_map.remove(&i) {
            encryptor.apply_multi(secrets, &mut buf);
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
