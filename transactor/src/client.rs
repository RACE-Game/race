//! Client-side in Transactor

use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::random::RandomMode;
use race_core::transport::TransportT;
use race_core::types::GameAccount;
use race_crypto::SecretState;

pub enum ClientMode {
    Transactor,
    Validator,
}

pub struct Client {
    mode: ClientMode,
    addr: String,
    rpc_client: HttpClient,
    secret_states: Vec<SecretState>,
}

impl Client {
    /// Create new transactor client.
    pub async fn new(addr: &str, init_account: GameAccount, transport: &dyn TransportT) -> Result<Self> {
        if !init_account.served {
            return Err(Error::GameNotServed);
        }

        let curr_transactor_account = transport
            .get_transactor_account(addr)
            .await
            .ok_or(Error::InvalidTransactorAddress)?;

        // Find the first transactor and connect to it.
        let transactor_addr = init_account
            .transactors
            .iter()
            .flatten()
            .nth(0)
            .ok_or(Error::CantFindTransactor)?;

        let (transactor_account, mode) = if addr.eq(transactor_addr) {
            (curr_transactor_account, ClientMode::Transactor)
        } else {
            let main_transactor_account = transport
                .get_transactor_account(transactor_addr)
                .await
                .ok_or(Error::InvalidTransactorAddress)?;
            (main_transactor_account, ClientMode::Validator)
        };

        let rpc_client = HttpClientBuilder::default()
            .build(transactor_account.endpoint)
            .or(Err(Error::InitializeRpcClientError))?;

        Ok(Self {
            addr: addr.to_owned(),
            mode,
            rpc_client,
            secret_states: Default::default(),
        })
    }

    fn randomize_and_mask(&self, context: &GameContext, random_id: usize) -> Result<()> {
        match self.mode {
            ClientMode::Transactor => (),
            ClientMode::Validator => (),
        };
        Ok(())
    }

    fn lock(&self, context: &GameContext, random_id: usize) -> Result<()> {
        match self.mode {
            ClientMode::Transactor => (),
            ClientMode::Validator => (),
        };
        Ok(())
    }

    fn decrypt(&self, context: &GameContext) -> Result<()> {
        Ok(())
    }

    /// Handle context changes.
    pub fn handle_context(&mut self, context: &mut GameContext) -> Result<()> {
        // Reset secret states when a new game starts.

        // Create corresponding secret state when new random state is created.
        let random_states = context.list_random_states();
        if random_states.len() > self.secret_states.len() {
            for i in random_states.len()..self.secret_states.len() {
                let rnd_st = &random_states[i];
                let secret_state = SecretState::from_random_state(rnd_st, RandomMode::Shuffler);
                self.secret_states.push(secret_state);
            }
        }

        // Randomization & Lock & Decryption
        for rnd_st in random_states.iter() {
            match rnd_st.status {
                race_core::random::CipherStatus::Ready => {
                    self.decrypt(context)?;
                },
                race_core::random::CipherStatus::Locking(ref lock_addr) => {
                    if lock_addr.eq(&self.addr) {
                        self.lock(context, rnd_st.id)?;
                    }
                }
                race_core::random::CipherStatus::Masking(ref mask_addr) => {
                    if mask_addr.eq(&self.addr) {
                        self.randomize_and_mask(context, rnd_st.id)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_create_secret_state() -> Result<()> {
    //     let ctx = GameContext::new(game_account);
    // }
}
