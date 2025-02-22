use super::*;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameReg {
    pub title: String, // max: 16 chars
    pub addr: SuiAddress,
    pub bundle_addr: SuiAddress,
    pub reg_time: u64,
}

impl From<GameReg> for GameRegistration {
    fn from(value: GameReg) -> Self {
        GameRegistration {
            title: value.title,
            addr: value.addr.to_string(),
            bundle_addr: value.bundle_addr.to_string(),
            reg_time: value.reg_time
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq, Clone))]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryObject {
    pub id: ObjectID,
    pub is_private: bool,
    pub size: u16, // capacity of the registration center
    pub owner: Option<SuiAddress>,
    pub games: Vec<GameReg>,
}

impl RegistryObject {
    pub fn into_account(self) -> RegistrationAccount {
        let RegistryObject { id, is_private, size, owner, games } = self;
        RegistrationAccount {
            addr: id.to_hex_uncompressed(),
            is_private,
            size,
            owner: owner.map(|addr| addr.to_string()),
            games: games.into_iter().map(Into::<GameRegistration>::into).collect()
        }
    }
}
