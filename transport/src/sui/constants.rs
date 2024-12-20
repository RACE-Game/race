// numbers for various limits
pub const MAX_IDENTIFIER_LEN: u8 = 16;
pub const MAX_NAME_LENGTH: u8 = 50;
pub const GAS_BUDGET: u64 = 5_500_001;
pub const MAX_GAME_NAME_LEN: usize = 50;

// fixed addresses
pub const PACKAGE_ID: &str = "0x2bb109d9c2c388fe699bf6961b3e36d7a4cdd073fb1317ff0e4fb826f56f8759";
pub const SUI_ACCOUNT: &str = "0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e";
pub const PUBLISHER: &str = "0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192";

// Common Coin addresses
pub const COIN_SUI_ADDR: &str = "0x2::sui::SUI";
pub const COIN_USDC_ADDR: &str = "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::usdc::USDC";

// names for modules, structs, enums and functions in the published move package
pub const RECIPIENT: &str = "recipient";
pub const SLOT_SHARE_STRUCT: &str = "RecipientSlotShare";
pub const RECIPIENT_BUILDER_FN: &str = "new_recipient_builder";
pub const RECIPIENT_SLOT_FN: &str = "create_recipient_slot";
pub const SLOT_SHARE_FN: &str = "create_slot_share";
pub const RECIPIENT_FN: &str = "create_recipient";
