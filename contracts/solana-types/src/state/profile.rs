#[cfg(feature = "program")]
use crate::constants::PROFILE_ACCOUNT_LEN;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
#[cfg(not(feature = "program"))]
use solana_sdk::pubkey::Pubkey;

// =======================================================
// ====================== PLAYER ACCOUNT =================
// =======================================================
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct PlayerState {
    pub is_initialized: bool,
    pub nick: String, // max: 16 chars
    pub pfp: Option<Pubkey>,
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

    fn create_player() -> PlayerState {
        let mut player = PlayerState::default();
        player.is_initialized = true;
        player.nick = "16-char_nickname".to_string();
        player.pfp = Some(Pubkey::default());
        player
    }

    #[test]
    fn test_player_account_len() -> anyhow::Result<()> {
        let player = create_player();
        let unpadded_len = get_instance_packed_len(&player)?;
        println!(
            "Player account len {}",
            unpadded_len
        );
        assert!(unpadded_len <= PROFILE_ACCOUNT_LEN);
        assert_eq!(unpadded_len, 54);
        Ok(())
    }

    #[test]
    fn test_deserialize_player() -> anyhow::Result<()> {
        let player = create_player();
        let unpadded_data = [1, 16, 0, 0, 0, 49, 54, 45, 99, 104, 97, 114, 95, 110, 105, 99, 107, 110, 97, 109, 101, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let player_ser = player.try_to_vec().unwrap();
        assert_eq!(player_ser, unpadded_data);
        Ok(())
    }
}
