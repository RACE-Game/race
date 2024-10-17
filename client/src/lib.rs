use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use race_api::error::{Error, Result};
use race_api::event::Event;
use race_api::random::{RandomState, RandomStatus};
use race_api::types::SecretShare;

use race_core::{
    connection::ConnectionT,
    context::GameContext,
    encryptor::EncryptorT,
    secret::SecretState,
    types::{
        AttachGameParams, Ciphertext, ClientMode, DecisionId, RandomId, SecretIdent, SecretKey,
        SubmitEventParams,
    },
};

use race_core::transport::TransportT;

/// Operation Ident
///
/// Each event can be recorded as one or more idents, we save these
/// idents to avoid duplicated submission.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum OpIdent {
    RandomSecret {
        random_id: RandomId,
        to_addr: Option<String>,
        index: usize,
    },
    AnswerSecret {
        decision_id: DecisionId,
    },
    Lock {
        random_id: RandomId,
    },
    Mask {
        random_id: RandomId,
    },
}

/// The client core for player, transactor and validator nodes.
///
/// It reads the updated context, generates events and sends them via
/// connection.
///
/// # Client Mode
///
/// Three modes are supported:
///
/// | Mode      | Randomize | Decisions | Decryption |
/// |-----------|-----------|-----------|------------|
/// | Client    | x         | o         | o          |
/// | Server    | o         | x         | o          |
/// | Validator | o         | x         | o          |
///
pub struct Client {
    pub encryptor: Arc<dyn EncryptorT>,
    pub transport: Arc<dyn TransportT>,
    pub connection: Arc<dyn ConnectionT>,
    pub game_addr: String,
    // The address of current node, player address or server address.
    pub addr: String,
    // The client mode could be player, validator or transactor.
    // Only player can send custom events.
    // Only transactor can send system events.
    pub mode: ClientMode,
    pub op_hist: BTreeSet<OpIdent>,
    pub secret_state: SecretState,
    pub id: u64,
}

impl Client {
    pub fn new(
        addr: String,
        game_addr: String,
        mode: ClientMode,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        connection: Arc<dyn ConnectionT>,
    ) -> Self {
        Self {
            addr,
            game_addr,
            mode,
            op_hist: BTreeSet::new(),
            secret_state: SecretState::new(encryptor.clone()),
            transport,
            encryptor,
            connection,
            id: 0,
        }
    }

    pub async fn attach_game(&self) -> Result<()> {
        let key = self.encryptor.export_public_key(None)?;
        self.connection
            .attach_game(
                &self.game_addr,
                AttachGameParams {
                    key,
                    signer: self.addr.clone(),
                },
            )
            .await
    }

    pub async fn submit_event(&self, event: Event) -> Result<()> {
        self.connection
            .submit_event(&self.game_addr, SubmitEventParams { event })
            .await
    }

    // pub async fn submit_custom_event<S: CustomEvent>(&self, custom_event: S) -> Result<()> {
    //     let event = Event::custom(&self.game_addr, &custom_event);
    //     self.connection
    //         .submit_event(&self.addr, SubmitEventParams { event })
    //         .await
    // }

    pub fn load_random_states(&mut self, game_context: &GameContext) -> Result<()> {
        for random_state in game_context.list_random_states().iter() {
            if !self.secret_state.is_random_loaded(random_state.id) {
                self.secret_state
                    .gen_random_secrets(random_state.id, random_state.size);
            }
        }
        Ok(())
    }

    pub fn answer_event(&mut self, decision_id: DecisionId, value: String) -> Result<Event> {
        let (ciphertext, digest) = self.secret_state.encrypt_answer(decision_id, value)?;
        Ok(Event::AnswerDecision {
            sender: self.id,
            decision_id,
            ciphertext,
            digest,
        })
    }

    pub async fn answer(&mut self, decision_id: DecisionId, value: String) -> Result<()> {
        let event = self.answer_event(decision_id, value)?;
        self.connection
            .submit_event(&self.game_addr, SubmitEventParams { event })
            .await?;
        Ok(())
    }

    pub fn handle_decision(&mut self, game_context: &GameContext) -> Result<Vec<Event>> {
        let mut shares = Vec::new();

        for state in game_context.list_decision_states() {
            println!("[CLET] Decision state {:?}", state);
            if state.get_owner().eq(&self.addr) {
                let secret = self
                    .secret_state
                    .get_decision_secret(state.id)
                    .ok_or(Error::MissingDecisionSecret(state.id))?;
                shares.push(SecretShare::new_for_answer(
                    state.id,
                    self.addr.clone(),
                    secret,
                ));
            }
        }
        if shares.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![Event::ShareSecrets {
                sender: self.id,
                shares,
            }])
        }
    }

    pub fn handle_random_waiting(&mut self, random_state: &RandomState) -> Result<Option<Event>> {
        let required_idents: Vec<SecretIdent> = random_state
            .list_required_secrets_by_from_addr(&self.addr)
            .into_iter()
            .filter_map(|idt| {
                let op_ident = OpIdent::RandomSecret {
                    random_id: idt.random_id,
                    to_addr: idt.to_addr.clone(),
                    index: idt.index,
                };
                if self.op_hist.contains(&op_ident) {
                    None
                } else {
                    Some(idt)
                }
            })
            .collect();

        let mut op_hist = Vec::new();

        let shares = required_idents
            .into_iter()
            .map(|idt| {
                let secret = self
                    .secret_state
                    .get_random_lock(idt.random_id, idt.index)?;
                op_hist.push(OpIdent::RandomSecret {
                    random_id: random_state.id,
                    index: idt.index,
                    to_addr: idt.to_addr.clone(),
                });
                Ok(SecretShare::new_for_random(
                    idt.random_id,
                    idt.index,
                    self.addr.clone(),
                    idt.to_addr,
                    secret,
                ))
            })
            .collect::<Result<Vec<SecretShare>>>()?;

        if shares.is_empty() {
            Ok(None)
        } else {
            let event = Event::ShareSecrets {
                sender: self.id,
                shares,
            };
            self.op_hist.extend(op_hist);
            Ok(Some(event))
        }
    }

    pub fn handle_random_masking(&mut self, random_state: &RandomState) -> Result<Option<Event>> {
        let op_ident = OpIdent::Mask {
            random_id: random_state.id,
        };

        if self.op_hist.contains(&op_ident) {
            return Ok(None);
        }

        let origin = random_state
            .ciphertexts
            .iter()
            .map(|c| c.ciphertext().to_owned())
            .collect();

        let mut masked = self
            .secret_state
            .mask(random_state.id, origin)
            .map_err(|e| Error::RandomizationError(e.to_string()))?;

        self.encryptor.shuffle(&mut masked);

        let event = Event::Mask {
            sender: self.id,
            random_id: random_state.id,
            ciphertexts: masked,
        };

        self.op_hist.insert(op_ident);

        Ok(Some(event))
    }

    pub fn handle_random_locking(&mut self, random_state: &RandomState) -> Result<Option<Event>> {
        let op_ident = OpIdent::Lock {
            random_id: random_state.id,
        };

        if self.op_hist.contains(&op_ident) {
            return Ok(None);
        }

        let origin = random_state
            .ciphertexts
            .iter()
            .map(|c| c.ciphertext().to_owned())
            .collect();

        let unmasked = self
            .secret_state
            .unmask(random_state.id, origin)
            .map_err(|e| Error::RandomizationError(e.to_string()))?;

        let locked = self
            .secret_state
            .lock(random_state.id, unmasked)
            .map_err(|e| Error::RandomizationError(e.to_string()))?;

        let event = Event::Lock {
            sender: self.id,
            random_id: random_state.id,
            ciphertexts_and_digests: locked,
        };

        self.op_hist.insert(op_ident);

        Ok(Some(event))
    }

    pub fn handle_randomization(&mut self, game_context: &GameContext) -> Result<Vec<Event>> {
        let mut events = vec![];
        for random_state in game_context.list_random_states().iter() {
            match random_state.status {
                RandomStatus::Ready => (),
                RandomStatus::Shared => (),
                RandomStatus::WaitingSecrets => {
                    if let Some(event) = self.handle_random_waiting(random_state)? {
                        events.push(event);
                    }
                }
                RandomStatus::Locking(ref addr) => {
                    if self.addr.eq(addr) {
                        if let Some(event) = self.handle_random_locking(random_state)? {
                            events.push(event);
                        }
                    }
                }
                RandomStatus::Masking(ref addr) => {
                    if self.addr.eq(addr) {
                        if let Some(event) = self.handle_random_masking(random_state)? {
                            events.push(event);
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    pub fn handle_updated_context(&mut self, ctx: &GameContext) -> Result<Vec<Event>> {
        self.id = ctx.addr_to_id(&self.addr)?;
        let events = match self.mode {
            ClientMode::Player => {
                self.load_random_states(ctx)?;
                self.handle_decision(ctx)?
            }
            ClientMode::Transactor | ClientMode::Validator => {
                self.load_random_states(ctx)?;
                self.handle_randomization(ctx)?
            }
        };
        Ok(events)
    }

    pub fn flush_secret_states(&mut self) {
        self.secret_state.clear();
        self.op_hist.clear();
    }

    /// Decrypt the ciphertexts with shared secrets.
    /// Return a mapping from mapping from indexes to decrypted value.
    pub fn decrypt(
        &self,
        ctx: &GameContext,
        random_id: RandomId,
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
