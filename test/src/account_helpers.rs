use crate::client_helpers::TestClient;
use borsh::BorshSerialize;
use race_core::types::{ClientMode, EntryType, GameAccount, GameBundle, PlayerJoin, ServerJoin};

pub fn test_game_addr() -> String {
    "TEST".into()
}

pub struct TestGameAccountBuilder {
    account: GameAccount,
}

impl Default for TestGameAccountBuilder {
    fn default() -> Self {
        let account = GameAccount {
            addr: "TEST".into(),
            title: "Unnamed".into(),
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
            checkpoint: vec![],
            checkpoint_access_version: 0,
        };
        TestGameAccountBuilder { account }
    }
}

impl TestGameAccountBuilder {
    pub fn new() -> Self {
        TestGameAccountBuilder::default()
    }

    pub fn from_account(account: &GameAccount) -> Self {
        TestGameAccountBuilder {
            account: account.clone(),
        }
    }

    pub fn build(self) -> GameAccount {
        self.account
    }

    pub fn with_max_players(mut self, max_players: u16) -> Self {
        if max_players < self.account.players.len() as _ {
            panic!("Invalid max_players specified, more players were added");
        }
        self.account.max_players = max_players;
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

    pub fn set_transactor(mut self, server: &TestClient) -> Self {
        if server.get_mode().ne(&ClientMode::Transactor) {
            panic!("A test client in TRANSACTOR Mode is required");
        }
        if self.account.transactor_addr.is_some() {
            panic!("Only one transactor is allowed");
        }
        if self
            .account
            .servers
            .iter()
            .find(|s| s.addr.eq(&server.get_addr()))
            .is_some()
        {
            panic!("Server already added")
        }
        self.account.transactor_addr = Some(server.get_addr());
        self.account.access_version += 1;
        self.account.servers.insert(
            0,
            ServerJoin {
                addr: server.get_addr(),
                endpoint: "".into(),
                access_version: self.account.access_version,
                verify_key: "".into(),
            },
        );
        self
    }

    pub fn add_validator(mut self, server: &TestClient) -> Self {
        if server.get_mode().ne(&ClientMode::Validator) {
            panic!("A test client in VALIDATOR Mode is required");
        }
        if self
            .account
            .servers
            .iter()
            .find(|s| s.addr.eq(&server.get_addr()))
            .is_some()
        {
            panic!("Server already added")
        }
        self.account.access_version += 1;
        self.account.servers.push(ServerJoin {
            addr: server.get_addr(),
            endpoint: "".into(),
            access_version: self.account.access_version,
            verify_key: "".into(),
        });
        self
    }

    pub fn add_player(self, player: &TestClient, deposit: u64) -> Self {
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
        player: &TestClient,
        deposit: u64,
        position: u16,
    ) -> Self {
        if self
            .account
            .players
            .iter()
            .find(|p| p.addr.eq(&player.get_addr()))
            .is_some()
        {
            panic!("Player already added")
        }
        if player.get_mode().ne(&ClientMode::Player) {
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
            addr: player.get_addr(),
            position,
            access_version: self.account.access_version,
            balance: deposit,
            verify_key: "".into(),
        });
        self
    }

    pub fn with_checkpoint<T: BorshSerialize>(self, checkpoint: T) -> Self {
        let checkpoint = checkpoint.try_to_vec().expect("Serialize data failed");
        self.with_checkpoint_vec(checkpoint)
    }

    pub fn with_checkpoint_vec(mut self, checkpoint: Vec<u8>) -> Self {
        self.account.checkpoint = checkpoint;
        self
    }

    pub fn with_data<T: BorshSerialize>(self, account_data: T) -> Self {
        let data = account_data.try_to_vec().expect("Serialize data failed");
        self.with_data_vec(data)
    }

    pub fn with_data_vec(mut self, data: Vec<u8>) -> Self {
        self.account.data_len = data.len() as _;
        self.account.data = data;
        self
    }
}

pub fn create_game_bundle_from_path(path: &str) -> GameBundle {
    let proj_root = project_root::get_project_root().expect("No project root found");
    let bundle_path = proj_root.join(path);
    let data = std::fs::read(bundle_path).expect("Can't read file");
    GameBundle {
        uri: "".to_string(),
        name: "FILE BUNDLE".to_string(),
        data,
    }
}
