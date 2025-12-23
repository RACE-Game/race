use super::*;

#[cfg_attr(test, derive(PartialEq, Default, Debug))]
#[derive(Deserialize, Serialize, Clone)]
pub struct PlayerProfileObject {
    pub id: SuiAddress,
    pub nick: String,
    pub pfp: Option<SuiAddress>,
    pub credentials: Vec<u8>,
}

impl PlayerProfileObject {
    pub fn into_profile(self) -> PlayerProfile {
        let pfp = match self.pfp {
            Some(addr) => Some(addr.to_string()),
            None => None
        };
        PlayerProfile {
            addr: self.id.to_string(),
            nick: self.nick,
            pfp,
            credentials: self.credentials,
        }
    }
}
