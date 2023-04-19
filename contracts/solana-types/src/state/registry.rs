#[cfg(feature = "program")]
use crate::constants::REGISTRY_ACCOUNT_LEN;
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

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub title: String, // max: 16 chars
    pub addr: Pubkey,
    pub bundle_addr: Pubkey,
    pub reg_time: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub is_private: bool,
    pub size: u16, // capacity of the registration center
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
    // pub padding: Box<Vec<u8>>,
    pub padding: Box<Vec<u8>>,
}

#[cfg(feature = "program")]
impl Padded for RegistryState {
    fn get_padding_mut(&mut self) -> Result<(usize, &mut Box<Vec<u8>>), ProgramError> {
        let packed_len = get_instance_packed_len(self)?;
        let current_padding_len = self.padding.len();
        let data_len = packed_len - current_padding_len;
        let needed_padding_len = Self::LEN - data_len;
        Ok((needed_padding_len, &mut self.padding))
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
        let mut state = RegistryState {
            is_initialized: true,
            is_private: false,
            size: 100,
            owner: Pubkey::new_unique(),
            games: Box::new(Vec::<GameReg>::with_capacity(100)),
            padding: Default::default(),
        };

        state.update_padding().unwrap();
        state
    }
    #[test]
    #[ignore]
    pub fn test_registry_account_len() -> anyhow::Result<()> {
        let mut registry = make_registry_state();
        println!("Current padding len {}", registry.padding.len());
        println!("Current padding cap {}", registry.padding.capacity());
        println!(
            "Registry account len {}",
            get_instance_packed_len(&registry)?
        );
        for i in 0..100 {
            let reg_game = GameReg {
                title: "gametitle_16_cha".to_string(),
                addr: Pubkey::new_unique(),
                reg_time: 1111111111111u64 + (i as u64),
                bundle_addr: Pubkey::new_unique(),
                // is_hidden: params.is_hidden,
            };
            registry.games.push(reg_game);
        }
        registry.update_padding()?;
        println!(
            "Registry account aligned len {}",
            get_instance_packed_len(&registry)?
        );
        println!("Aligned padding len {}", registry.padding.len());
        println!("Aligned padding cap {}", registry.padding.capacity());
        assert_eq!(1, 2);
        Ok(())
    }

    #[test]
    pub fn test_ser() -> anyhow::Result<()> {
        let mut state = make_registry_state();
        state.update_padding()?;
        println!("Game registry len {}", get_instance_packed_len(&state)?);
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state, &mut buf)?;
        Ok(())
    }

    #[test]
    pub fn test_deser() -> anyhow::Result<()> {
        let mut state = make_registry_state();
        state.update_padding()?;
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state.clone(), &mut buf)?;
        let deser = RegistryState::unpack(&buf)?;
        assert_eq!(deser, state);
        Ok(())
    }
}
