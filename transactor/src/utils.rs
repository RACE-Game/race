#[cfg(test)]
pub mod tests {
    use borsh::BorshSerialize;
    use race_core::types::GameAccount;

    pub fn game_account_with_empty_data() -> GameAccount {
        game_account_with_data(vec![])
    }

    pub fn game_account_with_account_data<S: BorshSerialize>(account_data: S) -> GameAccount {
        let data = account_data.try_to_vec().unwrap();
        game_account_with_data(data)
    }

    pub fn game_account_with_data(data: Vec<u8>) -> GameAccount {
        GameAccount {
            addr: mock_game_account_addr(),
            bundle_addr: mock_game_bundle_addr(),
            served: true,
            settle_version: 0,
            access_version: 0,
            players: vec![],
            data_len: data.len() as _,
            data,
            transactors: vec![],
            max_players: 2,
        }
    }

    pub fn mock_game_account_addr() -> String {
        "ACC ADDR".into()
    }

    pub fn mock_game_bundle_addr() -> String {
        "GAME ADDR".into()
    }
}
