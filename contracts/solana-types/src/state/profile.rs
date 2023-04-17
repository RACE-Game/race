#[cfg(feature = "program")]
use crate::constants::PROFILE_ACCOUNT_LEN;
#[cfg(feature = "program")]
use crate::state::Padded;
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
// ====================== PLAYER ACCOUNT =================
// =======================================================
#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct PlayerState {
    pub is_initialized: bool,
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
    #[ignore]
    pub fn test_player_account_len() -> anyhow::Result<()> {
        let mut player = create_player();
        println!(
            "Player account non-aligned len {}",
            get_instance_packed_len(&player)?
        );
        player.update_padding()?;
        println!(
            "Player account aligned len {}",
            get_instance_packed_len(&player)?
        );
        assert_eq!(get_instance_packed_len(&player)?, PlayerState::LEN);
        assert_eq!(1, 2);
        Ok(())
    }
}
