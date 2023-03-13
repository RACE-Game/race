use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    borsh::get_instance_packed_len,
    program_error::ProgramError,
    program_memory::sol_memcpy,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct PlayerJoin {
    pub addr: Pubkey,
    pub balance: u64,
    pub position: u32,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct ServerJoin {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct GameState {
    pub is_initialized: bool,
    pub title: String,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u8,
    pub data: Box<Vec<u8>>,
    pub players: Box<Vec<PlayerJoin>>,
    pub servers: Box<Vec<ServerJoin>>,
    pub padding: Box<Vec<u8>>,
}

impl GameState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        self.padding = Box::new(vec![0; padding_len]);
    }
}

impl IsInitialized for GameState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Sealed for GameState {}
impl Pack for GameState {
    const LEN: usize = 5000;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let r = GameState::try_from_slice(src)?;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    // #[test]
    // pub fn test() -> anyhow::Result<()> {
    //     let mut state = GameState::default();
    //     let s: String = "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEF".into();
    //     state.data = Box::new(vec![0; 1024]);
    //     state.title = s.clone();
    //     for i in 0..32 {
    //         state.players.push(PlayerJoin::default());
    //     }
    //     for i in 0..10 {
    //         let s = ServerJoin {
    //             addr: Pubkey::default(),
    //             endpoint: s.clone(),
    //             access_version: 0,
    //         };
    //         state.servers.push(s);
    //     }
    //     println!("len: {}", get_instance_packed_len(&state)?);
    //     assert_eq!(1, 2);
    //     Ok(())
    // }

    pub fn make_game_state() -> GameState {
        let mut state = GameState::default();
        state.is_initialized = true;
        for i in 0..16 {
            let mut p = PlayerJoin::default();
            p.position = i;
            state.players.push(p);
        }
        state
    }

    #[test]
    pub fn test_ser() -> anyhow::Result<()> {
        let mut state = make_game_state();
        state.update_padding();
        let mut buf = [0u8; GameState::LEN];
        GameState::pack(state, &mut buf)?;
        println!("{:?}", buf);
        Ok(())
    }

    #[test]
    pub fn test_deser() -> anyhow::Result<()> {
        let mut state = make_game_state();
        state.update_padding();
        let mut buf = [0u8; GameState::LEN];
        GameState::pack(state.clone(), &mut buf)?;
        let deser = GameState::unpack(&buf)?;
        assert_eq!(deser, state);
        Ok(())
    }
}
