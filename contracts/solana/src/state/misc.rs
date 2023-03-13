use crate::error::RaceError;
use solana_program::{
    program_error::ProgramError,
    program_pack::Pack,
};

pub const PUBKEY_LEN: usize = 32;
pub const NAME_LEN: usize = 16;
pub const OPTION_PREFIX_LEN: usize = 1;
pub const COVER_LEN: usize = 43;
pub const DESC_LEN: usize = 140;
pub const REG_SIZE: usize = 100;

pub trait PackOption: Pack {
    const OPT_LEN: usize = Self::LEN + 1;

    fn pack_option(opt: Option<Self>, dst: &mut [u8]) -> Result<(), ProgramError> {
        // let len = get_instance_packed_len(&Self)?;
        let len = Self::get_packed_len();
        Ok(match opt {
            None => dst[0..(len + 1)].fill(0),
            Some(s) => {
                dst[0] = 1;
                Self::pack(s, &mut dst[1..(len + 1)])?;
            }
        })
    }

    fn unpack_option(src: &[u8]) -> Result<Option<Self>, ProgramError> {
        let len = Self::get_packed_len();
        let o = src.get(0).ok_or(RaceError::UnpackOptionFailed)?;
        let s = src
            .get(1..(len + 1))
            .ok_or(RaceError::UnpackOptionFailed)?;
        Ok(match o {
            0 => None,
            1 => Some(Self::unpack_unchecked(s)?),
            _ => return Err(RaceError::UnpackOptionFailed.into()),
        })
    }
}
