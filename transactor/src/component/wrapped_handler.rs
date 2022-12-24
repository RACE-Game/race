use std::path::PathBuf;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::{GameContext, GameStatus, Player, PlayerStatus};
use race_core::error::{Error, Result};
use race_core::event::{Event, RandomizeOp, SecretIdent};
use race_core::transport::TransportT;
use race_core::types::GameAccount;
use wasmer::{imports, Instance, Module, Store, TypedFunction};

pub struct WrappedHandler {
    store: Store,
    instance: Instance,
}

impl WrappedHandler {
    /// Load WASM bundle by game address
    pub async fn load_by_addr(addr: &str, transport: &dyn TransportT) -> Result<Self> {
        let mut store = Store::default();
        let game_bundle = transport.get_game_bundle(addr).await.ok_or(Error::GameBundleNotFound)?;
        let module = Module::from_binary(&store, &game_bundle.data).or(Err(Error::MalformedGameBundle))?;
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self { store, instance })
    }

    /// Load WASM bundle by relative path
    /// This function is used for testing.
    #[allow(dead_code)]
    pub fn load_by_path(path: PathBuf) -> Result<Self> {
        let mut store = Store::default();
        let module = Module::from_file(&store, path).expect("Fail to load module");
        let import_object = imports![];
        let instance = Instance::new(&mut store, &module, &import_object).expect("Init failed");
        Ok(Self { store, instance })
    }

    pub fn init_state(&mut self, context: &mut GameContext, init_account: &GameAccount) {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
        memory.grow(&mut self.store, 10).expect("Failed to grow");
        let init_state: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "init_state")
            .expect("Failed to get function");
        let mem_view = memory.view(&self.store);
        let context_bs = context.try_to_vec().unwrap();
        let init_account_bs = init_account.try_to_vec().unwrap();
        let mut offset = 1u64;
        mem_view
            .write(offset as _, &context_bs)
            .expect("Failed to write context");
        offset += context_bs.len() as u64;
        mem_view
            .write(offset as _, &init_account_bs)
            .expect("Failed to write init account");
        let len = init_state
            .call(&mut self.store, context_bs.len() as _, init_account_bs.len() as _)
            .expect("Handle event error");
        println!("Len: {:?}", len);
        let mut buf = Vec::with_capacity(len as _);
        buf.resize(len as _, 0);
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
    }

    fn custom_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        let memory = self.instance.exports.get_memory("memory").expect("Get memory failed");
        let handle_event: TypedFunction<(u32, u32), u32> = self
            .instance
            .exports
            .get_typed_function(&self.store, "handle_event")
            .expect("Failed to get function");
        let mem_view = memory.view(&self.store);
        let context_bs = context.try_to_vec().unwrap();
        let event_bs = event.try_to_vec().unwrap();
        let mut offset = 1u64;
        mem_view
            .write(offset as _, &context_bs)
            .expect("Failed to write context");
        offset += context_bs.len() as u64;
        mem_view.write(offset as _, &event_bs).expect("Failed to write event");
        let len = handle_event
            .call(&mut self.store, context_bs.len() as _, event_bs.len() as _)
            .expect("Handle event error");
        println!("Len: {:?}", len);
        let mut buf = Vec::with_capacity(len as _);
        buf.resize(len as _, 0);
        println!("buf: {:?}", buf);
        println!("buf len: {:?}", buf.len());
        mem_view.read(1u64, &mut buf).unwrap();
        *context = GameContext::try_from_slice(&buf).unwrap();
        Ok(())
    }

    fn general_handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        match event {
            Event::Ready { sender } => {
                if let Some(p) = context.players.iter_mut().find(|p| p.addr.eq(sender)) {
                    p.status = PlayerStatus::Ready;
                    Ok(())
                } else {
                    Err(Error::InvalidPlayerAddress)
                }
            }

            Event::ShareSecrets {
                sender: _,
                secret_ident,
                secret_data,
            } => {
                if context.shared_secrets.contains_key(secret_ident) {
                    Err(Error::DuplicatedSecretSharing)
                } else {
                    context.shared_secrets.insert(secret_ident.clone(), secret_data.clone());
                    Ok(())
                }
            }

            Event::Randomize {
                sender,
                random_id,
                op,
                ciphertexts,
            } => {
                match op {
                    RandomizeOp::Lock => {
                        let rnd_st = context.get_mut_random_state(*random_id)?;
                        rnd_st.lock(sender, vec![])?;
                    }
                    RandomizeOp::Mask => {
                        let rnd_st = context.get_mut_random_state(*random_id)?;
                        rnd_st.mask(sender, ciphertexts.clone())?;
                    }
                }
                Ok(())
            }

            Event::RandomnessReady => Ok(()),

            Event::Join {
                player_addr,
                balance: amount,
            } => {
                if let Some(_) = context.players.iter().find(|p| p.addr.eq(player_addr)) {
                    Err(Error::PlayerAlreadyJoined)
                } else {
                    let p = Player::new(player_addr, *amount);
                    context.players.push(p);
                    Ok(())
                }
            }
            Event::Leave { player_addr: _ } => {
                // This event is for game handler
                if context.allow_leave {
                    Ok(())
                } else {
                    Err(Error::CantLeave)
                }
            }

            Event::GameStart => {
                context.status = GameStatus::Running;
                Ok(())
            }

            Event::WaitTimeout => Ok(()),

            Event::DrawRandomItems {
                sender,
                random_id,
                indexes,
            } => {
                Ok(())
            }

            Event::ActionTimeout { player_addr: _ } => {
                // This event is for game handler
                Ok(())
            }

            Event::SecretsReady => Ok(()),
            _ => Ok(()),
        }
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: &Event) -> Result<()> {
        self.general_handle_event(context, event)?;
        self.custom_handle_event(context, event)
    }
}

#[cfg(test)]
mod tests {
    use race_core::{context::DispatchEvent, types::GameAccount};

    use super::*;

    #[derive(BorshSerialize)]
    pub struct MinimalAccountData {
        counter_value_default: u64,
    }

    fn make_game_account() -> GameAccount {
        let data = MinimalAccountData {
            counter_value_default: 42,
        }
        .try_to_vec()
        .unwrap();
        GameAccount {
            addr: "ACC ADDR".into(),
            bundle_addr: "GAME ADDR".into(),
            settle_serial: 0,
            access_serial: 0,
            players: vec![],
            data_len: data.len() as _,
            data,
            transactors: vec![],
            max_players: 2,
        }
    }

    fn make_wrapped_handler() -> WrappedHandler {
        WrappedHandler::load_by_path("../target/wasm32-unknown-unknown/release/race_example_minimal.wasm".into())
            .unwrap()
    }

    #[test]
    fn test_init_state() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        hdlr.init_state(&mut ctx, &game_account);
        assert_eq!("{\"counter_value\":42,\"counter_players\":0}", ctx.state_json);
    }

    #[test]
    fn test_handle_event() {
        let mut hdlr = make_wrapped_handler();
        let game_account = make_game_account();
        let mut ctx = GameContext::new(&game_account);
        let event = Event::Join {
            player_addr: "FAKE_ADDR".into(),
            balance: 1000,
        };
        hdlr.init_state(&mut ctx, &game_account);
        println!("ctx: {:?}", ctx);
        hdlr.handle_event(&mut ctx, &event).unwrap();
        assert_eq!("{\"counter_value\":42,\"counter_players\":1}", ctx.state_json);
    }
}
