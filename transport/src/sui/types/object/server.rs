use super::*;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ServerObject {
    pub addr: SuiAddress,
    pub owner: SuiAddress,
    pub endpoint: String, // max: 50 chars
}

impl ServerObject {
    pub fn into_account(self) -> ServerAccount {
        let ServerObject { owner, endpoint, .. } = self;
        ServerAccount {
            addr: owner.to_string(),
            endpoint
        }
    }
}
