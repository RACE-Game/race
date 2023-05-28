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
    // #[ignore]
    pub fn test_player_account_len() -> anyhow::Result<()> {
        let player = create_player();
        println!(
            "Player account non-aligned len {}",
            get_instance_packed_len(&player)?
        );
        println!(
            "Player account aligned len {}",
            get_instance_packed_len(&player)?
        );
        println!("data: {:?}", player.try_to_vec().unwrap());
        Ok(())
    }
}
