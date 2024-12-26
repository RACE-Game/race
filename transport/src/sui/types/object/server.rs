use super::*;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ServerObject {
    pub addr: SuiAddress,
    pub owner: SuiAddress,
    pub endpoint: String, // max: 50 chars
}
