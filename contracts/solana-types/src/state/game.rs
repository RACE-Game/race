use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    borsh::get_instance_packed_len,
    program_error::ProgramError,
    program_memory::sol_memcpy,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
#[cfg(feature = "sdk")]
use solana_sdk::pubkey::Pubkey;
use crate::{constants::{GAME_ACCOUNT_LEN, SERVER_ACCOUNT_LEN}, types::VoteType};

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct PlayerJoin {
    pub addr: Pubkey,
    pub balance: u64,
    pub position: u32,
    pub access_version: u64,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct ServerJoin {
    pub addr: Pubkey,
    pub endpoint: String,
    pub access_version: u64,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct Vote {
    pub voter: Pubkey,
    pub votee: Pubkey,
    pub vote_type: VoteType,
}

// State of on-chain GameAccount
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize, Debug)]
pub struct GameState {
    pub is_initialized: bool,
    pub title: String,
    pub bundle_addr: Pubkey,
    pub stake_addr: Pubkey,
    pub owner: Pubkey,
    pub token_addr: Pubkey,
    pub transactor_addr: Option<Pubkey>,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u8,
    pub data_len: u32,
    pub data: Box<Vec<u8>>,
    pub players: Box<Vec<PlayerJoin>>,
    pub servers: Box<Vec<ServerJoin>>,
    pub votes: Box<Vec<Vote>>,
    pub unlock_time: Option<u64>,
    pub padding: Box<Vec<u8>>,
}

#[cfg(feature = "program")]
impl GameState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        self.padding = Box::new(vec![0; padding_len]);
    }
}

#[cfg(feature = "program")]
impl IsInitialized for GameState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[cfg(feature = "program")]
impl Sealed for GameState {}

#[cfg(feature = "program")]
impl Pack for GameState {
    const LEN: usize = GAME_ACCOUNT_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let result = GameState::try_from_slice(src)?;
        Ok(result)
    }
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct PlayerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub chips: u64,
    pub nick: String, // max: 16 chars
    pub pfp: Option<Pubkey>,
    pub padding: Vec<u8>,
}

#[cfg(feature = "program")]
impl PlayerState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        self.padding = vec![0; padding_len];
    }
}

#[cfg(feature = "program")]
impl IsInitialized for PlayerState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[cfg(feature = "program")]
impl Sealed for PlayerState {}

#[cfg(feature = "program")]
impl Pack for PlayerState {
    const LEN: usize = 98;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let result = PlayerState::try_from_slice(src)?;
        Ok(result)
    }
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String, // max: 50 chars
    pub padding: Vec<u8>,
}

#[cfg(feature = "program")]
impl ServerState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        self.padding = vec![0; padding_len];
    }
}

#[cfg(feature = "program")]
impl IsInitialized for ServerState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[cfg(feature = "program")]
impl Sealed for ServerState {}

#[cfg(feature = "program")]
impl Pack for ServerState {
    const LEN: usize = SERVER_ACCOUNT_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let result = ServerState::try_from_slice(src)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn create_player() -> PlayerState {
        let mut player = PlayerState::default();
        player.nick = "a_16-char_nicknm".to_string();
        player.pfp = Some(Pubkey::new_unique());
        player
    }

    #[test]
    pub fn test_player_account_len() -> anyhow::Result<()> {
        let mut player = create_player();
        println!("Player account non-aligned len {}", get_instance_packed_len(&player)?);
        player.update_padding();
        println!("Player account aligned len {}", get_instance_packed_len(&player)?);
        assert_eq!(get_instance_packed_len(&player)?, PlayerState::LEN); // 98
        assert_eq!(1, 2);
        Ok(())
    }

    fn create_server() -> ServerState {
        let mut server = ServerState::default();
        server.addr = Pubkey::new_unique();
        server.owner = Pubkey::new_unique();
        server.endpoint = "https------------------------------".to_string();
        server.update_padding();
        server
    }

    #[test]
    pub fn test_server_account_len() -> anyhow::Result<()> {
        let server = create_server();
        println!("Server account len {}", get_instance_packed_len(&server)?); // 104
        assert_eq!(1, 2);
        Ok(())
    }

    #[test]
    pub fn test_state_len() -> anyhow::Result<()> {
        let mut state = GameState::default();
        let s: String = "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDE".into();
        state.data = Box::new(vec![0; 1024]);
        state.title = s.clone();
        for _ in 0..32 {
            state.players.push(PlayerJoin::default());
        }
        for _ in 0..10 {
            let s = ServerJoin {
                addr: Pubkey::default(),
                endpoint: s.clone(),
                access_version: 0,
            };
            state.servers.push(s);
        }
        println!("len: {}", get_instance_packed_len(&state)?);
        Ok(())
    }

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
    #[ignore]
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
