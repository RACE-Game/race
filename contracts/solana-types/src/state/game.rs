#[cfg(feature = "program")]
use crate::constants::GAME_ACCOUNT_LEN;
use crate::types::VoteType;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    program_error::ProgramError,
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
    pub position: u16,
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
    // game name displayed on chain
    pub title: String,
    // addr to the game core logic program on Arweave
    pub bundle_addr: Pubkey,
    // addr to the account that holds all players' deposits
    pub stake_account: Pubkey,
    // game owner who created this game account
    pub owner: Pubkey,
    // mint id of the token used for game
    pub token_mint: Pubkey,
    // minimum deposit for joining the game
    pub min_deposit: u64,
    // maximum deposit allowed in game
    pub max_deposit: u64,
    // addr of the first server joined the game
    pub transactor_addr: Option<Pubkey>,
    // a serial number, increased by 1 after each PlayerJoin or ServerJoin
    pub access_version: u64,
    // a serial number, increased by 1 after each settlement
    pub settle_version: u64,
    // game size
    pub max_players: u16,
    // game players
    pub players: Box<Vec<PlayerJoin>>,
    // game servers (max: 10)
    pub servers: Box<Vec<ServerJoin>>,
    // length of game-specific data
    pub data_len: u32,
    // serialized data of game-specific data such as sb/bb in Texas Holdem
    pub data: Box<Vec<u8>>,
    // game votes
    pub votes: Box<Vec<Vote>>,
    // unlock time
    pub unlock_time: Option<u64>,
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

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        Self::deserialize(&mut src).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[cfg(test)]
mod tests {

    use solana_program::borsh::get_instance_packed_len;

    use super::*;

    #[test]
    #[ignore]
    pub fn test_state_len() -> anyhow::Result<()> {
        let mut state = GameState::default();
        state.is_initialized = true;
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
        let game_state_len = get_instance_packed_len(&state)?;
        println!("Game state len: {}", game_state_len);
        assert_eq!(GAME_ACCOUNT_LEN, game_state_len);
        Ok(())
    }

    #[test]
    pub fn test_usize() {
        let state = PlayerJoin {
            addr: Pubkey::default(),
            balance: 1,
            position: 1,
            access_version: 1,
        };
        println!("data: {:?}", state.try_to_vec().unwrap());
        assert_eq!(1, 2);
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
        let state = make_game_state();
        let mut buf = [0u8; GameState::LEN];
        GameState::pack(state, &mut buf)?;
        println!("{:?}", buf);
        Ok(())
    }

    #[test]
    pub fn test_deser() -> anyhow::Result<()> {
        let state = make_game_state();
        let mut buf = [0u8; GameState::LEN];
        GameState::pack(state.clone(), &mut buf)?;
        let deser = GameState::unpack(&buf)?;
        assert_eq!(deser, state);
        Ok(())
    }
}
