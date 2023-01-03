use borsh::BorshSerialize;
use race_core::types::{TransactorAccount, GameAccount};

pub fn transactor_account() -> TransactorAccount {
    TransactorAccount {
        addr: transactor_account_addr(),
        owner_addr: transactor_owner_addr(),
        endpoint: transactor_endpoint(),
    }
}

pub fn game_account_with_empty_data() -> GameAccount {
    game_account_with_data(vec![])
}

pub fn game_account_with_account_data<S: BorshSerialize>(account_data: S) -> GameAccount {
    let data = account_data.try_to_vec().unwrap();
    game_account_with_data(data)
}

pub fn game_account_with_data(data: Vec<u8>) -> GameAccount {
    GameAccount {
        addr: game_account_addr(),
        bundle_addr: game_bundle_addr(),
        settle_version: 0,
        access_version: 0,
        players: vec![],
        data_len: data.len() as _,
        data,
        transactor_addr: Some(transactor_account_addr()),
        server_addrs: vec![transactor_account_addr()],
        max_players: 2,
    }
}

pub fn game_account_addr() -> String {
    "ACC ADDR".into()
}

pub fn game_bundle_addr() -> String {
    "GAME ADDR".into()
}

pub fn transactor_account_addr() -> String {
    "TRANSACTOR ADDR".into()
}

pub fn transactor_owner_addr() -> String {
    "TRANSACTOR OWNER".into()
}

pub fn transactor_endpoint() -> String {
    "TRANSACTOR ENDPOINT".into()
}
