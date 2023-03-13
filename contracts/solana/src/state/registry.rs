use std::mem::size_of;

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use super::{misc::REG_SIZE, PackOption};

pub const TOURNAMENT_REG_ACCOUNT_LEN: usize = (1 + TournamentReg::LEN) * REG_SIZE;

#[derive(Debug)]
pub struct TournamentReg {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub reg_time: u32,
    pub start_time: u32,
    pub is_hidden: bool,
}

impl PackOption for TournamentReg {}

impl Sealed for TournamentReg {}

impl Pack for TournamentReg {
    const LEN: usize = size_of::<Pubkey>()
        + size_of::<Pubkey>()
        + size_of::<u32>()
        + size_of::<u32>()
        + size_of::<bool>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, TournamentReg::LEN];
        let (pubkey, mint, reg_time, start_time, is_hidden) = mut_array_refs![
            dst,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<u32>(),
            size_of::<u32>(),
            size_of::<bool>()
        ];
        pubkey.copy_from_slice(self.pubkey.as_ref());
        mint.copy_from_slice(self.mint.as_ref());
        *reg_time = u32::to_le_bytes(self.reg_time);
        *start_time = u32::to_le_bytes(self.start_time);
        is_hidden[0] = self.is_hidden as u8;
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let src = array_ref![src, 0, TournamentReg::LEN];
        let (pubkey, mint, reg_time, start_time, is_hidden) = array_refs![
            src,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<u32>(),
            size_of::<u32>(),
            size_of::<bool>()
        ];
        Ok(Self {
            pubkey: Pubkey::new_from_array(*pubkey),
            mint: Pubkey::new_from_array(*mint),
            reg_time: u32::from_le_bytes(*reg_time),
            start_time: u32::from_le_bytes(*start_time),
            is_hidden: is_hidden[0] == 1,
        })
    }
}

pub const GAME_REG_ACCOUNT_LEN: usize = (1 + GameReg::LEN) * REG_SIZE;

#[derive(Debug)]
pub struct GameReg {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub is_hidden: bool,
}

impl Sealed for GameReg {}

impl PackOption for GameReg {}

impl Pack for GameReg {
    const LEN: usize = size_of::<Pubkey>() + size_of::<Pubkey>() + size_of::<bool>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, GameReg::LEN];
        let (pubkey, mint, is_hidden) = mut_array_refs![
            dst,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<bool>()
        ];
        pubkey.copy_from_slice(self.pubkey.as_ref());
        mint.copy_from_slice(self.mint.as_ref());
        is_hidden[0] = self.is_hidden as u8;
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let src = array_ref![src, 0, GameReg::LEN];
        let (pubkey, mint, is_hidden) = array_refs![
            src,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<bool>()
        ];
        Ok(Self {
            pubkey: Pubkey::new_from_array(*pubkey),
            mint: Pubkey::new_from_array(*mint),
            is_hidden: is_hidden[0] == 1,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RegCenter {
    pub is_initialized: bool,
    pub is_private: bool,
    pub owner: Pubkey,
    pub tournament_reg: Pubkey,
    pub game_reg: Pubkey,
}

impl IsInitialized for RegCenter {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Sealed for RegCenter {}

impl Pack for RegCenter {
    const LEN: usize = size_of::<bool>()
        + size_of::<bool>()
        + size_of::<Pubkey>()
        + size_of::<Pubkey>()
        + size_of::<Pubkey>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, RegCenter::LEN];
        let (is_initialized, is_private, owner, tournament_reg, game_reg) = mut_array_refs![
            dst,
            1,
            1,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<Pubkey>()
        ];
        is_initialized[0] = self.is_initialized as u8;
        is_private[0] = self.is_private as u8;
        owner.copy_from_slice(self.owner.as_ref());
        tournament_reg.copy_from_slice(self.tournament_reg.as_ref());
        game_reg.copy_from_slice(self.game_reg.as_ref());
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let src = array_ref![src, 0, RegCenter::LEN];
        let (is_initialized, is_private, owner, tournament_reg, game_reg) = array_refs![
            src,
            1,
            1,
            size_of::<Pubkey>(),
            size_of::<Pubkey>(),
            size_of::<Pubkey>()
        ];
        Ok(Self {
            is_initialized: is_initialized[0] == 1,
            is_private: is_private[0] == 1,
            owner: Pubkey::new_from_array(*owner),
            tournament_reg: Pubkey::new_from_array(*tournament_reg),
            game_reg: Pubkey::new_from_array(*game_reg),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tournament_reg() {
        let pubkey = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let is_hidden = true;
        let start_time = 200;
        let reg_time = 200;
        let state = TournamentReg {
            pubkey,
            mint,
            reg_time,
            start_time,
            is_hidden,
        };
        let mut buf = [0; TournamentReg::LEN];
        TournamentReg::pack(state, &mut buf).unwrap();

        let unpacked = TournamentReg::unpack_unchecked(&buf).unwrap();
        assert_eq!(pubkey, unpacked.pubkey);
        assert_eq!(mint, unpacked.mint);
        assert_eq!(is_hidden, unpacked.is_hidden);
        assert_eq!(reg_time, unpacked.reg_time);
        assert_eq!(start_time, unpacked.start_time);
    }
}
