use crate::constants::*;
use borsh::BorshSerialize;
use race_core::types::{GameAccount, PlayerJoin, ServerAccount, ServerJoin};

pub fn player_account_addr(idx: usize) -> String {
    PLAYER_ADDRS[idx].to_owned()
}

pub fn server_account_addr(idx: usize) -> String {
    SERVER_ADDRS[idx].to_owned()
}

pub fn game_account_addr() -> String {
    TEST_GAME_ACCOUNT_ADDR.into()
}

pub fn game_bundle_addr() -> String {
    TEST_GAME_BUNDLE_ADDR.into()
}

pub fn transactor_account_addr() -> String {
    TEST_TRANSACTOR_ACCOUNT_ADDR.into()
}

pub fn transactor_owner_addr() -> String {
    TEST_TRANSACTOR_OWNER_ADDR.into()
}

pub fn transactor_endpoint() -> String {
    TEST_TRANSACTOR_ENDPOINT.into()
}

pub struct TestGameAccountBuilder {
    account: GameAccount,
}

impl Default for TestGameAccountBuilder {
    fn default() -> Self {
        let account = GameAccount {
            addr: game_account_addr(),
            title: "Unnamed".into(),
            bundle_addr: game_bundle_addr(),
            owner_addr: "8ZVzTrut4TMXjRod2QRFBqGeyLzfLNnQEj2jw3q1sBqu".to_string(),
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
            max_deposit: 2,
            min_deposit: 1,
            token_addr: "RACE5fnTKB9obGtCusArTQ6hhdNXAtf3HarvJM17rxJ".to_string(),
        };
        TestGameAccountBuilder { account }
    }
}

/// A tuple of (address, deposit, position, access_version),
type TestPlayerInfo = (String, u64, u16, u64);

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

    pub fn default_players() {}

    pub fn add_servers(mut self, num_of_servers: usize) -> Self {
        if num_of_servers > 3 {
            panic!("num_of_servers must be less than or equal to 3");
        }

        for addr in SERVER_ADDRS
            .iter()
            .skip(self.account.servers.len())
            .take(num_of_servers)
        {
            if self.account.transactor_addr.is_none() {
                self.account.transactor_addr = Some(addr.to_string());
            }
            self.account.access_version += 1;
            self.account.servers.push(ServerJoin::new(
                addr.clone(),
                "".into(),      // server addr
                self.account.access_version,
                "".into()       // verify key
            ));
        }
        self
    }

    pub fn add_players(mut self, num_of_players: usize) -> Self {
        if num_of_players > 6 {
            panic!("num_of_players must be less than or equal to 6");
        }

        for (i, addr) in PLAYER_ADDRS
            .iter()
            .enumerate()
            .skip(self.account.players.len())
            .take(num_of_players)
        {
            self.account.access_version += 1;
            self.account.players.push(PlayerJoin {
                addr: addr.to_string(),
                position: i as u16,
                access_version: self.account.access_version,
                balance: DEFAULT_DEPOSIT_AMOUNT,
                verify_key: "".into(),
            });
        }
        self
    }

    pub fn with_players(mut self, players: &[TestPlayerInfo]) -> Self {
        for (addr, amount, position, access_version) in players.iter() {
            self.account.players.push(PlayerJoin {
                addr: addr.to_owned(),
                position: *position,
                access_version: *access_version,
                balance: *amount,
                verify_key: "".into(),
            });
        }
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

pub fn transactor_account() -> ServerAccount {
    ServerAccount {
        addr: transactor_account_addr(),
        endpoint: transactor_endpoint(),
    }
}
