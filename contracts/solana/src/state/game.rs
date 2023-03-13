use borsh::{BorshSerialize, BorshDeserialize};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    borsh::{get_instance_packed_len, get_packed_len},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use crate::error::RaceError;

// TODO: Import the follwoing consts and trat from misc
// use super::misc::{...}
// Constants
pub const PUBKEY_LEN: usize = 32;
pub const NAME_LEN: usize = 16;
pub const OPTION_PREFIX_LEN: usize = 1;
pub const COVER_LEN: usize = 43;
pub const DESC_LEN: usize = 140;
pub const REG_SIZE: usize = 100;
pub const MAX_PLAYERS_NUM: usize = 9;

pub trait PackOption: Pack {
    const OPT_LEN: usize = Self::LEN + 1;

    fn pack_option(opt: Option<Self>, dst: &mut [u8]) -> Result<(), ProgramError> {
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

#[derive(BorshSerialize, BorshDeserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Player {
    // 32 + 8 + 4 + 1 = 45
    pub pubkey: Pubkey,
    pub chips: u64,
    pub buyin_serial: u32,
    pub rebuy: u8,
}

impl Sealed for Player {}

impl PackOption for Player {}

impl Pack for Player {
    const LEN: usize = PUBKEY_LEN + 8 + 4 + 1;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Player::LEN];
        let (pubkey, chips, buyin_serial, rebuy) = mut_array_refs![dst, PUBKEY_LEN, 8, 4, 1];
        pubkey.copy_from_slice(self.pubkey.as_ref());
        *chips = u64::to_le_bytes(self.chips);
        *buyin_serial = u32::to_le_bytes(self.buyin_serial);
        *rebuy = u8::to_le_bytes(self.rebuy);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let data = array_ref![src, 0, Player::LEN];
        let (pubkey, chips, buyin_serial, rebuy) = array_refs![data, PUBKEY_LEN, 8, 4, 1];
        Ok(Player {
            pubkey: Pubkey::new_from_array(*pubkey),
            chips: u64::from_le_bytes(*chips),
            buyin_serial: u32::from_le_bytes(*buyin_serial),
            rebuy: u8::from_le_bytes(*rebuy),
        })
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct GameState {
    pub is_initialized: bool,
    // incremented by 1 for each buyin
    pub buyin_serial: u32,
    // incremented by 1 for each settlement
    pub settle_serial: u32,
    pub stake_account_pubkey: Pubkey,
    pub mint_pubkey: Pubkey,
    pub players: Vec<Player>,
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    // table size
    pub size: u8,
    // pub game_type: GameType,
    pub transactor_pubkey: Pubkey,
    pub owner_pubkey: Pubkey,
    pub transactor_rake: u16,
    // owner rake (per thousand)
    pub owner_rake: u16,

    // TODO: Add the following fields
    // pub ante: u64,
    // minimum buyin amount
    // a pubkey for scene NFT
    // pub scene_pubkey: Pubkey,
    // all player states
    // the account holds all buyin assets
    // game's on-chain status
    // pub status: GameStatus,
    // pub bonus_pubkey: Option<Pubkey>,
    // name
    // pub name: [u8; NAME_LEN],
}

impl Sealed for GameState {}

impl IsInitialized for GameState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for GameState {
    const LEN: usize = 1                        // is_initialized: bool
        + 4                                     // buyin_serial: u32
        + 4                                     // settle_serial: u32
        + PUBKEY_LEN                            // state account pubkey: Pubkey
        + PUBKEY_LEN                            // mint pubkey
        + (MAX_PLAYERS_NUM * Player::LEN)       // All players
        + 8                                     // sb
        + 8                                     // bb
        + 8                                     // buyin
        + 1                                     // size
        + PUBKEY_LEN                            // transactor pubkey
        + PUBKEY_LEN                            // owner pubkey
        + 2                                     // transactor rake
        + 2                                     // owner rake
        // TODO:
        // + GameType::LEN             // game type
        // + 8                  // ante
        // + GameStatus::LEN                     // status
        // + OPTION_PREFIX_LEN + PUBKEY_LEN      // bonus
        // + NAME_LEN                            // name
        ;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, GameState::LEN];
        let (is_initialized, data_len, data) = mut_array_refs![dst, 1, 8, GameState::LEN - 9];
        is_initialized[0] = self.is_initialized as u8;

        // let (
        //     is_initialized,
        //     buyin_serial,
        //     settle_serial,
        //     stake_account_pubkey,
        //     mint_pubkey,
        //     players,
        //     sb,
        //     bb,
        //     buyin,
        //     size,
        //     transactor_pubkey,
        //     owner_pubkey,
        //     transactor_rake,
        //     owner_rake,
        //     // TODO: Add following vars
        //     // scene_pubkey,
        //     // players,
        //     // ante,
        //     // game_type,
        //     // status,
        //     // bonus_opt,
        //     // bonus_pubkey,
        //     // name,
        // ) = mut_array_refs![
        //     dst,
        //     1,
        //     4,
        //     4,
        //     PUBKEY_LEN,
        //     PUBKEY_LEN,
        //     (MAX_PLAYERS_NUM * (1 + Player::LEN)),
        //     8,
        //     8,
        //     8,
        //     1,
        //     PUBKEY_LEN,
        //     PUBKEY_LEN,
        //     2,
        //     2
        // ];

        // scene_pubkey.copy_from_slice(self.scene_pubkey.as_ref());
        // stake_account_pubkey.copy_from_slice(self.stake_account_pubkey.as_ref());
        // mint_pubkey.copy_from_slice(self.mint_pubkey.as_ref());
        // *ante = u64::to_le_bytes(self.ante);
        // *sb = u64::to_le_bytes(self.sb);
        // *bb = u64::to_le_bytes(self.bb);
        // *buyin = u64::to_le_bytes(self.buyin);
        // *buyin_serial = u32::to_le_bytes(self.buyin_serial);
        // *settle_serial = u32::to_le_bytes(self.settle_serial);
        // *size = u8::to_le_bytes(self.size);
        // GameType::pack(self.game_type, game_type).unwrap();
        // transactor_pubkey.copy_from_slice(self.transactor_pubkey.as_ref());
        // owner_pubkey.copy_from_slice(self.owner_pubkey.as_ref());
        // *transactor_rake = u16::to_le_bytes(self.transactor_rake);
        // *owner_rake = u16::to_le_bytes(self.owner_rake);
        // name.copy_from_slice(self.name.as_ref());
        // GameStatus::pack(self.status, status).unwrap();

        // if let Some(bonus_pubkey_src) = self.bonus_pubkey {
        //     bonus_opt[0] = 1;
        //     bonus_pubkey.copy_from_slice(bonus_pubkey_src.as_ref());
        // } else {
        //     bonus_opt[0] = 0;
        //     bonus_pubkey.copy_from_slice(Pubkey::default().as_ref());
        // }

        // for i in 0..MAX_PLAYERS_NUM {
        //     let player = array_mut_ref![players, i * (Player::LEN + 1), Player::LEN + 1];
        //     Player::pack_option(self.players[i], player).unwrap();
        // }
    }

     fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, GameState::LEN];
        let (
            is_initialized,
            buyin_serial,
            settle_serial,
            // scene_pubkey,
            raw_players,
            stake_account_pubkey,
            mint_pubkey,
            // ante,
            sb,
            bb,
            buyin,
            size,
            // game_type,
            transactor_pubkey,
            owner_pubkey,
            transactor_rake,
            owner_rake,
            // status,
            // bonus_opt,
            // bonus_pubkey,
            // name,
        ) = array_refs![
            src,
            1,
            4,
            4,
            // PUBKEY_LEN,
            (MAX_PLAYERS_NUM * (1 + Player::LEN)),
            PUBKEY_LEN,
            PUBKEY_LEN,
            // 8,
            8,
            8,
            8,
            1,
            // GameType::LEN,
            PUBKEY_LEN,
            PUBKEY_LEN,
            2,
            2
            // GameStatus::LEN,
            // OPTION_PREFIX_LEN,
            // PUBKEY_LEN,
            // NAME_LEN
        ];
        let mut players: [Option<Player>; MAX_PLAYERS_NUM] = [None; MAX_PLAYERS_NUM];
        for i in 0..MAX_PLAYERS_NUM {
            let player = array_ref![raw_players, i * (Player::LEN + 1), Player::LEN + 1];
            players[i] = Player::unpack_option(player)?;
        }
        // let bonus_pubkey = if bonus_opt[0] == 1 {
        //     Some(Pubkey::new_from_array(*bonus_pubkey))
        // } else {
        //     None
        // };
        Ok(GameState {
            is_initialized: is_initialized[0] == 1,
            buyin_serial: u32::from_le_bytes(*buyin_serial),
            settle_serial: u32::from_le_bytes(*settle_serial),
            // scene_pubkey: Pubkey::new_from_array(*scene_pubkey),
            players,
            stake_account_pubkey: Pubkey::new_from_array(*stake_account_pubkey),
            mint_pubkey: Pubkey::new_from_array(*mint_pubkey),
            sb: u64::from_le_bytes(*sb),
            bb: u64::from_le_bytes(*bb),
            // ante: u64::from_le_bytes(*ante),
            buyin: u64::from_le_bytes(*buyin),
            size: u8::from_le_bytes(*size),
            // game_type: GameType::unpack_unchecked(game_type)?,
            transactor_pubkey: Pubkey::new_from_array(*transactor_pubkey),
            owner_pubkey: Pubkey::new_from_array(*owner_pubkey),
            transactor_rake: u16::from_le_bytes(*transactor_rake),
            owner_rake: u16::from_le_bytes(*owner_rake),
            // status: GameStatus::unpack_unchecked(status)?,
            // bonus_pubkey,
            // name: *name,
        })
    }
}
