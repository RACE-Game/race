// numbers for various limits
pub const MAX_IDENTIFIER_LEN: u8 = 16;
pub const MAX_NAME_LENGTH: u8 = 50;
pub const GAS_BUDGET: u64 = 5_500_001;

// fixed addresses
pub const PACKAGE_ID: &str = "0xd943d83d6a80cdc98cb83b414d468b8ec35d3f46fca194093e3e78b69bc96da5";
pub const SUI_ACCOUNT: &str = "0xd1204296954a3db409ecd2fd35c2ee750f12dafb1088cb1656566078fc46ad6e";
pub const PUBLISHER: &str = "0x7a1f6dc139d351b41066ea726d9b53670b6d827a0745d504dc93e61a581f7192";

// Common Coin addresses
pub const COIN_SUI_ADDR: &str = "0x2::sui::SUI";
pub const COIN_USDC_ADDR: &str = "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::usdc::USDC";

// modules and functions in the published move package
pub const RECIPIENT: &str = "recipient";
pub const NEW_RECIPIENT_BUILDER: &str = "new_recipient_builder";
pub const CREATE_RECIPIENT_SLOT: &str = "create_recipient_slot";
pub const CREATE_SLOT_SHARE: &str = "create_slot_share";
pub const CREATE_RECIPIENT: &str = "create_recipient";
