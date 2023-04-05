use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    borsh::get_instance_packed_len,
    program_error::ProgramError,
    program_memory::sol_memcpy,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use crate::constants::REGISTRY_ACCOUNT_LEN;

#[cfg(feature = "sdk")]
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub title: String,          // max: 30 chars
    pub addr: Pubkey,
    pub bundle_addr: Pubkey,
    pub reg_time: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub is_private: bool,
    pub addr: Pubkey,
    pub size: u16, // capacity of the registration center
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
    pub padding: Box<Vec<u8>>,
}

#[cfg(feature = "program")]
impl RegistryState {
    pub fn update_padding(&mut self) {
        let data_len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - data_len;
        self.padding = Box::new(vec![0u8; padding_len]);
    }
}

#[cfg(feature = "program")]
impl IsInitialized for RegistryState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[cfg(feature = "program")]
impl Sealed for RegistryState {}
#[cfg(feature = "program")]
impl Pack for RegistryState {
    const LEN: usize = REGISTRY_ACCOUNT_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        match RegistryState::try_from_slice(src) {
            Ok(result) => Ok(result),
            Err(_) => Ok(RegistryState::default()),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn make_registry_state() -> RegistryState {
        let mut state = RegistryState::default();
        state.is_initialized = true;
        for _i in 0..100 {
            let g = GameReg::default();
            // g.title = "gametitle_16_cha".to_string();
            state.games.push(g);
        }
        state
    }
    #[test]
    pub fn test_registry_account_len() -> anyhow::Result<()> {
        let mut registry = make_registry_state();
        println!("Registry account non-alighed len {}", get_instance_packed_len(&registry)?);
        registry.update_padding();
        println!("Registry account aligned len {}", get_instance_packed_len(&registry)?);
        assert_eq!(1, 2);
        Ok(())
    }

    #[test]
    pub fn test_ser() -> anyhow::Result<()> {
        let mut state = make_registry_state();
        state.update_padding();
        println!("Game registry len {}", get_instance_packed_len(&state)?);
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state, &mut buf)?;
        Ok(())
    }

    #[test]
    pub fn test_deser() -> anyhow::Result<()> {
        let mut state = make_registry_state();
        state.update_padding();
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state.clone(), &mut buf)?;
        let deser = RegistryState::unpack(&buf)?;
        assert_eq!(deser, state);
        Ok(())
    }
}
