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

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String,               // max: 50 chars
    pub padding: Vec<u8>,
}

#[cfg(feature = "program")]
impl ServerState {
    pub fn update_padding(&mut self) {
        let len = get_instance_packed_len(self).unwrap();
        let padding_len = Self::LEN - len;
        self.padding = vec![0; padding_len];
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
    const LEN: usize = 108;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        sol_memcpy(dst, &data, data.len());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let result = ServerState::try_from_slice(src)?;
        Ok(result)
    }
}
