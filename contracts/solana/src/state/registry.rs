use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey, borsh::get_instance_packed_len, program_memory::sol_memcpy, program_error::ProgramError, msg,
};

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, BorshDeserialize, BorshSerialize, Clone)]
pub struct GameReg {
    pub addr: Pubkey,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct RegistryState {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub games: Box<Vec<GameReg>>,
    pub padding: Box<Vec<u8>>,
}

impl RegistryState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        msg!("Padding len: {}", padding_len);
        self.padding = Box::new(vec![0; padding_len]);
    }
}

impl IsInitialized for RegistryState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Sealed for RegistryState {}
impl Pack for RegistryState {
    const LEN: usize = 2000;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        match RegistryState::try_from_slice(src) {
            Ok(r) => Ok(r),
            Err(_) => Ok(RegistryState::default())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    pub fn make_registry_state() -> RegistryState {
        let mut state = RegistryState::default();
        state.is_initialized = true;
        for _i in 0..16 {
            let g = GameReg::default();
            state.games.push(g);
        }
        state
    }

    #[test]
    pub fn test_ser() -> anyhow::Result<()> {
        let mut state = make_registry_state();
        state.update_padding();
        let mut buf = [0u8; RegistryState::LEN];
        RegistryState::pack(state, &mut buf)?;
        println!("{:?}", buf);
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

    #[test]
    pub fn foo() -> anyhow::Result<()> {
        let buf = [0u8; RegistryState::LEN];
        let state = RegistryState::unpack_unchecked(&buf)?;
        Ok(())
    }
}
