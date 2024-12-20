use super::*;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Deserialize, Serialize, Default, Debug)]
pub struct PlayerObject {
    pub nick: String, // max: 16 chars
    pub pfp: Option<SuiAddress>,
}
