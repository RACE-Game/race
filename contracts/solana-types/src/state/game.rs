#[cfg(feature = "program")]
use crate::state::Padded;
use crate::types::VoteType;
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

// =======================================================
// ====================== GAME ACCOUNT ===================
// =======================================================
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
impl Padded for GameState {
    fn get_padding_mut(&mut self) -> Result<(usize, &mut Box<Vec<u8>>), ProgramError> {
        let packed_len = get_instance_packed_len(self)?;
        let current_padding_len = self.padding.len();
        let data_len = packed_len - current_padding_len;
        let needed_padding_len = Self::LEN - data_len;
        Ok((needed_padding_len, &mut self.padding))
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

// =======================================================
// ====================== PLAYER ACCOUNT =================
// =======================================================
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct PlayerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub chips: u64,
    pub nick: String, // max: 16 chars
    pub pfp: Option<Pubkey>,
    pub padding: Box<Vec<u8>>,
}

#[cfg(feature = "program")]
impl Padded for PlayerState {
    fn get_padding_mut(&mut self) -> Result<(usize, &mut Box<Vec<u8>>), ProgramError> {
        let packed_len = get_instance_packed_len(self)?;
        let current_padding_len = self.padding.len();
        let data_len = packed_len - current_padding_len;
        let needed_padding_len = Self::LEN - data_len;
        Ok((needed_padding_len, &mut self.padding))
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
    const LEN: usize = PROFILE_ACCOUNT_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let result = PlayerState::try_from_slice(src)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn create_player() -> PlayerState {
        let mut player = PlayerState::default();
        player.nick = "16-char_nickname".to_string();
        player.pfp = Some(Pubkey::new_unique());
        player
    }

    #[test]
    pub fn test_player_account_len() -> anyhow::Result<()> {
        let mut player = create_player();
        println!(
            "Player account non-aligned len {}",
            get_instance_packed_len(&player)?
        );
        player.update_padding();
        println!(
            "Player account aligned len {}",
            get_instance_packed_len(&player)?
        );
        assert_eq!(get_instance_packed_len(&player)?, PlayerState::LEN); // 98
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
        println!("Game state len: {}", get_instance_packed_len(&state)?);
        assert_eq!(1, 2);
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
