// numbers for various limits
pub const MAX_IDENTIFIER_LEN: u8 = 16;
pub const MAX_NAME_LENGTH: u8 = 50;
pub const GAS_BUDGET: u64 = 5_500_001;

// fixed addresses
pub const PACKAGE_ID: &str = "0x2c89d15410778dcc0d489a972ce15a85374fe0f7033cbebe9903b1b01ff6a252";
pub const SUI_ACCOUNT: &str = "0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e";
pub const SUI_COIN_ADDR: &str = "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI";
pub const PUBLISHER: &str = "0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192";

// modules and functions in the published move package
pub const RECIPIENT: &str = "recipient";
pub const NEW_RECIPIENT_BUILDER: &str = "new_recipient_builder";
pub const BUILD_SLOT_TYPE: &str = "build_slot_type";
pub const CREATE_RECIPIENT_SLOT: &str = "create_recipient_slot";
pub const CREATE_RECIPIENT: &str = "create_recipient";
