#[cfg(feature = "program")]
use crate::constants::SERVER_ACCOUNT_LEN;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
#[cfg(not(feature = "program"))]
use solana_sdk::pubkey::Pubkey;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Default, Debug)]
pub struct ServerState {
    pub is_initialized: bool,
    pub addr: Pubkey,
    pub owner: Pubkey,
    pub endpoint: String, // max: 50 chars
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
    pub fn test_server_account_len() -> anyhow::Result<()> {
        let mut server = ServerState::default();
        server.addr = Pubkey::new_unique();
        server.owner = Pubkey::new_unique();
        server.endpoint = "https------------------------------".to_string();
        println!("Server account len {}", get_instance_packed_len(&server)?); // 104
        Ok(())
    }
}
