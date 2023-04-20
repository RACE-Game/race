#[cfg(feature = "program")]
use crate::constants::REGISTRY_ACCOUNT_LEN;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    program_error::ProgramError,
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

    fn make_registry_state() -> RegistryState {
        let state = RegistryState {
            is_initialized: true,
            is_private: false,
            size: 100,
            owner: Pubkey::new_unique(),
            games: Box::new(Vec::<GameReg>::with_capacity(100)),
        };

        state
    }
    #[test]
    #[ignore]
    pub fn test_registry_account_len() -> anyhow::Result<()> {
        let mut registry = make_registry_state();
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
        println!(
            "Registry account aligned len {}",
            get_instance_packed_len(&registry)?
        );
        assert_eq!(1, 2);
        Ok(())
    }

    #[test]
    pub fn test_ser() -> anyhow::Result<()> {
        let state = make_registry_state();
        println!("Game registry len {}", get_instance_packed_len(&state)?);
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state, &mut buf)?;
        Ok(())
    }

    #[test]
    pub fn test_deser() -> anyhow::Result<()> {
        let state = make_registry_state();
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state.clone(), &mut buf)?;
        let deser = RegistryState::unpack(&buf)?;
        assert_eq!(deser, state);
        Ok(())
    }
}
