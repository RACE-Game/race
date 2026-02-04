use super::*;

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ServerObject {
    pub addr: SuiAddress,
    pub owner: SuiAddress,
    pub endpoint: String, // max: 50 chars
    pub credentials: Vec<u8>,
}

impl ServerObject {
    pub fn into_account(self) -> ServerAccount {
        let ServerObject { owner, endpoint, credentials, .. } = self;
        ServerAccount {
            addr: owner.to_string(),
            endpoint,
            credentials,
        }
    }
}
