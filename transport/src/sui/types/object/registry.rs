use super::*;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameReg {
    pub title: String, // max: 16 chars
    pub addr: SuiAddress,
    pub bundle_addr: SuiAddress,
    pub reg_time: u64,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Default, Deserialize, Serialize)]
pub struct RegistryObject {
    pub is_private: bool,
    pub size: u16, // capacity of the registration center
    pub owner: SuiAddress,
    pub games: Vec<GameReg>,
}
