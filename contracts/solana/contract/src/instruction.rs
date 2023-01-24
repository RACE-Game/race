use borsh::{BorshDeserialize, BorshSerialize};
use race_core::types::{CloseGameAccountParams, CreateGameAccountParams};
use solana_program::pubkey::Pubkey;
#[derive(BorshSerialize, BorshDeserialize)]
pub enum RaceContractInstruction {
    CreateGame(CreateGameAccountParams),
    CloseGame(CloseGameAccountParams),
}

pub fn game_account_seed(address: Pubkey) -> Vec<u8> {
    let res = format!("game_account-{address}");
    solana_program::hash::hash(res.as_bytes())
        .to_bytes()
        .to_vec()
}
