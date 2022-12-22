use race_core::types::GameAccount;

pub fn mock_game_account() -> GameAccount {
    GameAccount {
        addr: "FAKE GAME ACCOUNT ADDR".into(),
        bundle_addr: "FAKE GAME BUNDLE ADDR".into(),
        settle_serial: 0,
        access_serial: 0,
        max_players: 2,
        transactors: vec![],
        players: vec![],
        data_len: 0,
        data: vec![],
    }
}
