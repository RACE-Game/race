use std::collections::HashMap;

use crate::client_helpers::TestClient;
use crate::misc::{test_game_addr, test_game_title};
use crate::prelude::{AsGameContextRef, TestHandler};
use borsh::BorshSerialize;
use race_api::engine::GameHandler;
use race_api::event::Event;
use race_api::prelude::InitAccount;
use race_core::error::Result;
use race_core::checkpoint::{CheckpointOffChain, CheckpointOnChain, VersionedData};
use race_core::random::RandomState;
use race_core::types::{ClientMode, EntryLock, EntryType, GameAccount, GameSpec, PlayerDeposit, PlayerJoin, ServerJoin, DepositStatus};
use race_core::context::{DispatchEvent, EventEffects, GameContext, Versions};

pub struct TestContext<H>
where
    H: GameHandler,
{
    context: GameContext,
    account: GameAccount,
    handler: TestHandler<H>,
}

impl<H: GameHandler> TestContext<H> {

    /// Let a player join this game with a certain deposit.
    /// Return the Join event and the Deposit event.
    pub fn join(&mut self, player: &mut TestClient, deposit: u64) -> (Event, Event) {
        self.join_multi(vec![(player, deposit)])
    }

    /// Let a player leave this game.
    /// Return the Leave event.
    pub fn leave(&mut self, player: &TestClient) -> Event {
        Event::Leave { player_id: player.id() }
    }

    /// Let multiple players join this game.
    /// Return the Join event and the Deposit event.
    pub fn join_multi(&mut self, players_and_deposits: Vec<(&mut TestClient, u64)>) -> (Event, Event) {
        let mut players = vec![];
        let mut deposits = vec![];
        for (test_client, deposit) in players_and_deposits.into_iter() {
            let (player, deposit) = test_client
                .join(&mut self.context, &mut self.account, deposit)
                .expect("Add player to TestContext");

            players.push(player);
            deposits.push(deposit);
        }
        (Event::Join { players }, Event::Deposit{ deposits })
    }

    /// Handle one event and return the effects.
    pub fn handle_event(&mut self, event: &Event) -> Result<EventEffects> {
        self.handler.handle_event(&mut self.context, event)
    }

    /// Handle one event and all events generated after it, until the next event satisfies
    /// the prediction.  Return the next event and the last effects.
    pub fn handle_event_until(
        &mut self,
        event: &Event,
        mut clients: Vec<&mut TestClient>,
        event_pred: impl for<'a> Fn(Option<&'a Event>) -> bool,
    ) -> Result<(Option<Event>, EventEffects)> {
        let mut events_queue = vec![event.clone()];
        let mut effects = EventEffects::default();

        while !event_pred(events_queue.first()) {
            let this_event = &events_queue.remove(0);
            println!("* Handle event: {}", this_event);

            effects = self.handler.handle_event(&mut self.context, this_event)?;

            // Handle the following dispatched event and clients events.
            if let Some(dispatch) = self.take_dispatch() {
                if dispatch.timeout == self.context.get_timestamp() {
                    events_queue.push(dispatch.event);
                }
            }

            // Handle the following clients event.
            for client in clients.iter_mut() {
                let client_events = client.handle_updated_context(&mut self.context)?;
                events_queue.extend_from_slice(&client_events);
                if effects.checkpoint.is_some() {
                    client.flush_secret_state();
                }
            }
        }

        let next_event = if events_queue.is_empty() {
            None
        } else {
            Some(events_queue.remove(0))
        };
        Ok((next_event, effects))
    }

    /// Handle one event and all events generated after it, until there's no more event.
    /// Return the last effects.
    pub fn handle_event_until_no_events(
        &mut self,
        event: &Event,
        clients: Vec<&mut TestClient>,
    ) -> Result<EventEffects> {
        let (_, effects) = self.handle_event_until(&event, clients, |e|{ e.is_none() })?;
        Ok(effects)
    }

    /// Like `handle_event` but pass in the current dispatched event.
    pub fn handle_dispatch(&mut self) -> Result<EventEffects> {
        let event = &self.take_dispatch().expect("No dispatch event").event;
        self.handle_event(event)
    }

    /// Like `handle_event_until_no_events` but start with the current dispatched event.
    pub fn handle_dispatch_until_no_events(
        &mut self,
        clients: Vec<&mut TestClient>,
    ) -> Result<EventEffects> {
        let event = &self.take_dispatch().expect("No dispatch event").event;
        self.handle_event_until_no_events(event, clients)
    }

    /// Like `handle_event_until` but start with the current dispatched event.
    pub fn handle_dispatch_until(
        &mut self,
        clients: Vec<&mut TestClient>,
        event_pred: impl for<'a> Fn(Option<&'a Event>) -> bool,
    ) -> Result<(Option<Event>, EventEffects)> {
        let event = &self.take_dispatch().expect("No dispatch event").event;
        self.handle_event_until(event, clients, event_pred)
    }

    /// Handle multiple events and return the effects of the last event.
    pub fn handle_multiple_events(&mut self, events: &[Event]) -> Result<EventEffects> {
        let mut e = EventEffects::default();
        for event in events {
            e = self.handle_event(event)?;
        }
        Ok(e)
    }

    pub fn init_account(&self) -> Result<InitAccount> {
        Ok(self.context.init_account())
    }

    pub fn state(&self) -> &H {
        self.handler.state()
    }

    pub fn state_mut(&mut self) -> &mut H {
        self.handler.state_mut()
    }

    pub fn random_state(&mut self, random_id: usize) -> Result<&RandomState> {
        self.context.get_random_state(random_id)
    }

    pub fn random_state_mut(&mut self, random_id: usize) -> Result<&mut RandomState> {
        self.context.get_random_state_mut(random_id)
    }

    pub fn set_random_result(&mut self, random_id: usize, result: HashMap<usize, String>) {
        self.handler.set_random_result(random_id, result);
    }

    pub fn take_dispatch(&mut self) -> Option<DispatchEvent> {
        self.context.take_dispatch()
    }

    pub fn current_dispatch(&self) -> Option<DispatchEvent> {
        self.context.get_dispatch().clone()
    }

    pub fn client_events(&self, client: &mut TestClient) -> Result<Vec<Event>> {
        client.handle_updated_context(self)
    }

    pub fn client_decrypt(&self, client: &TestClient, random_id: usize) -> Result<HashMap<usize, String>> {
        client.decrypt(self, random_id)
    }
}

pub struct TestContextBuilder {
    account: GameAccount,
}

impl Default for TestContextBuilder {
    fn default() -> Self {
        let account = GameAccount {
            addr: test_game_addr(),
            title: test_game_title(),
            bundle_addr: "".into(),
            owner_addr: "".into(),
            settle_version: 0,
            access_version: 0,
            players: vec![],
            data_len: 0,
            data: vec![],
            transactor_addr: None,
            servers: vec![],
            votes: vec![],
            unlock_time: None,
            max_players: 6,
            deposits: vec![],
            recipient_addr: "".into(),
            entry_type: EntryType::default(),
            token_addr: "".into(),
            checkpoint_on_chain: None,
            entry_lock: EntryLock::Open,
            bonuses: vec![],
            balances: vec![],
        };
        TestContextBuilder { account }
    }
}

impl TestContextBuilder {
    /// Initialize by calling handler's `init_state` without checkpoint.
    pub fn build_with_init_state<H: GameHandler>(self) -> Result<(TestContext<H>, EventEffects)> {
        let mut context = GameContext::try_new(&self.account, None)
            .expect("Create game context with initial state api");
        context.set_node_ready(self.account.access_version);

        let (handler, event_effects) = TestHandler::<H>::init_state(&mut context)?;

        Ok((
            TestContext {
                context,
                account: self.account,
                handler,
            },
            event_effects,
        ))
    }

    /// Initialize with handler's checkpoint state.
    pub fn build_with_checkpoint<H: GameHandler>(
        mut self,
        checkpoint: &H,
    ) -> Result<(TestContext<H>, EventEffects)> {
        let mut checkpoint_on_chain = CheckpointOnChain::default();
        let mut checkpoint_off_chain = CheckpointOffChain::default();

        checkpoint_on_chain.access_version = self.account.access_version;
        self.account.checkpoint_on_chain = Some(checkpoint_on_chain);
        let spec = GameSpec {
            game_addr: self.account.addr.clone(),
            game_id: 0,
            bundle_addr: self.account.bundle_addr.clone(),
            max_players: self.account.max_players,
        };

        checkpoint_off_chain.data.insert(
            0,
            VersionedData {
                id: 0,
                versions: Versions::new(0, 1),
                data: borsh::to_vec(checkpoint).expect("Failed to serialize checkpoint"),
                sha: vec![],
                game_spec: spec,
                dispatch: None,
                bridge_events: vec![],
            },
        );

        let mut context = GameContext::try_new(&self.account, Some(checkpoint_off_chain))
            .expect("Create game context with checkpoint state");

        let (handler, event_effects) = TestHandler::<H>::init_state(&mut context)?;

        Ok((
            TestContext {
                context,
                account: self.account,
                handler,
            },
            event_effects,
        ))
    }

    pub fn with_max_players(mut self, max_players: u16) -> Self {
        if max_players < self.account.players.len() as _ {
            panic!("Invalid max_players specified, more players were added");
        }
        self.account.max_players = max_players;
        self
    }

    pub fn with_deposit_amount(mut self, amount: u64) -> Self {
        self.account.entry_type = EntryType::Ticket {
            amount,
        };
        self
    }

    pub fn with_deposit_range(mut self, min: u64, max: u64) -> Self {
        if max < min {
            panic!("Invalid deposit value, the max must be greater than the min");
        }
        self.account.entry_type = EntryType::Cash {
            max_deposit: max,
            min_deposit: min,
        };
        self
    }

    pub fn set_transactor(mut self, server: &mut TestClient) -> Self {
        if server.mode().ne(&ClientMode::Transactor) {
            panic!("A test client in TRANSACTOR Mode is required");
        }
        if self.account.transactor_addr.is_some() {
            panic!("Only one transactor is allowed");
        }
        if self
            .account
            .servers
            .iter()
            .find(|s| s.addr.eq(&server.addr()))
            .is_some()
        {
            panic!("Server already added")
        }
        self.account.transactor_addr = Some(server.addr());
        self.account.access_version += 1;
        self.account.servers.insert(
            0,
            ServerJoin {
                addr: server.addr(),
                endpoint: "".into(),
                access_version: self.account.access_version,
                verify_key: "".into(),
            },
        );
        server.set_id(self.account.access_version);
        self
    }

    pub fn add_player(self, player: &mut TestClient, deposit: u64) -> Self {
        let mut position = None;
        for i in 0..self.account.max_players {
            if self
                .account
                .players
                .iter()
                .find(|p| p.position == i)
                .is_some()
            {
                continue;
            } else {
                position = Some(i);
                break;
            }
        }
        if let Some(position) = position {
            self.add_player_with_position(player, deposit, position)
        } else {
            panic!("Can't add player, game account is full");
        }
    }

    pub fn add_player_with_position(
        mut self,
        player: &mut TestClient,
        deposit: u64,
        position: u16,
    ) -> Self {
        if self
            .account
            .players
            .iter()
            .find(|p| p.addr.eq(&player.addr()))
            .is_some()
        {
            panic!("Player already added")
        }
        if player.mode().ne(&ClientMode::Player) {
            panic!("A test client in PLAYER mode is required");
        }
        self.account.access_version += 1;
        for p in self.account.players.iter() {
            if p.position == position {
                panic!("Player position occupied");
            }
        }
        if position >= self.account.max_players {
            panic!("Player position occupied");
        }
        self.account.players.push(PlayerJoin {
            addr: player.addr(),
            position,
            access_version: self.account.access_version,
            verify_key: "".into(),
        });
        self.account.deposits.push(PlayerDeposit {
            addr: player.addr(),
            amount: deposit,
            access_version: self.account.access_version,
            settle_version: self.account.settle_version,
            status: DepositStatus::Accepted,
        });
        player.set_id(self.account.access_version);
        self
    }

    pub fn with_data<T: BorshSerialize>(self, account_data: T) -> Self {
        let data = borsh::to_vec(&account_data).expect("Serialize data failed");
        self.with_data_vec(data)
    }

    pub fn with_data_vec(mut self, data: Vec<u8>) -> Self {
        self.account.data_len = data.len() as _;
        self.account.data = data;
        self
    }
}


impl<H: GameHandler> AsGameContextRef for TestContext<H> {
    fn as_game_context_ref(&self) -> &GameContext {
        &self.context
    }
}
