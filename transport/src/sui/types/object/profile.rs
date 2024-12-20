use serde::{Serialize, Deserialize};
use sui_sdk::types::base_types::SuiAddress;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Default, Debug)]
pub struct PlayerObject {
    pub nick: String, // max: 16 chars
    pub pfp: Option<SuiAddress>,
}
