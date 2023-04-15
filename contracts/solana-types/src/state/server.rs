#[cfg(feature = "program")]
use crate::state::Padded;
#[cfg(feature = "program")]
use crate::constants::SERVER_ACCOUNT_LEN;
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
use borsh::{BorshDeserialize, BorshSerialize};

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String, // max: 50 chars
    pub padding: Box<Vec<u8>>,
}

#[cfg(feature = "program")]
impl Padded for ServerState {
    fn get_padding_mut(&mut self) -> Result<(usize, &mut Box<Vec<u8>>), ProgramError> {
        let packed_len = get_instance_packed_len(self)?;
        let current_padding_len = self.padding.len();
        let data_len = packed_len - current_padding_len;
        let needed_padding_len = Self::LEN - data_len;
        Ok((needed_padding_len, &mut self.padding))
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

    #[test]
    pub fn test_server_account_len() -> anyhow::Result<()> {
        let mut server = ServerState::default();
        server.addr = Pubkey::new_unique();
        server.owner = Pubkey::new_unique();
        server.endpoint = "https------------------------------".to_string();
        server.update_padding();
        println!("Server account len {}", get_instance_packed_len(&server)?); // 104
        assert_eq!(1, 2);
        Ok(())
    }
}
