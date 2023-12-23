use race_api::event::Event;
use race_api::types::GamePlayer;

type NewPlayerSpec<'a> = (&'a str, u16, u64);

pub fn sync_new_players(addr_pos_balance_list: &[NewPlayerSpec], _access_version: u64) -> Event {
    let mut new_players: Vec<GamePlayer> = Vec::default();

    for (addr, pos, balance) in addr_pos_balance_list.iter() {
        if new_players.iter().find(|p| p.addr.eq(addr)).is_some() {
            panic!("Duplicated address: {}", addr)
        }
        if *balance == 0 {
            panic!("Zero balance: {}", addr)
        }
        if new_players.iter().find(|p| p.position.eq(pos)).is_some() {
            panic!("Duplicated position: {} at {}", addr, pos)
        }
        new_players.push(GamePlayer {
            addr: addr.to_string(),
            position: *pos,
            balance: *balance,
        });
    }

    Event::Sync { new_players }
}
