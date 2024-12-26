// TODO: make imports more specific?
use super::*;
// Helper functions for parsing/converting string to Sui types
pub(crate) fn new_identifier(literal: &str) -> Result<Identifier> {
    Identifier::new(literal)
        .map_err(|e| Error::TransportError(e.to_string()))
}

pub(crate) fn parse_sui_addr(value: &str) -> Result<SuiAddress> {
    SuiAddress::from_str(value)
        .map_err(|e| Error::TransportError(e.to_string()))
}

pub(crate) fn parse_account_addr(value: &str) -> Result<AccountAddress> {
    AccountAddress::from_str(value)
        .map_err(|e| Error::TransportError(e.to_string()))
}

pub(crate) fn parse_option_addr(value: Option<String>) -> Result<Option<SuiAddress>> {
    match value {
        Some(val) => {
            let addr = parse_sui_addr(&val)?;
            Ok(Some(addr))
        }
        None => {
            Ok(None)
        }
    }
}

pub(crate) fn parse_object_id(value: &str) -> Result<ObjectID> {
    ObjectID::from_hex_literal(value)
        .map_err(|e| Error::TransportError(e.to_string()))
}

// Use the underlying (or inner) bytes of address/id to convert it to `SuiAddress`
pub(crate) fn to_sui_addr(bytes: [u8; 32]) -> Result<SuiAddress> {
    SuiAddress::from_bytes(bytes).map_err(|e| Error::TransportError(e.to_string()))
}

// Convert a `SuiAddress` to `AccountAddress` using its inner bytes
pub(crate) fn to_account_addr(addr: SuiAddress) -> Result<AccountAddress> {
    let bytes = addr.to_inner();
    AccountAddress::from_bytes(bytes)
        .map_err(|e| Error::TransportError(e.to_string()))
}

// In case sometimes we do not need the `0x` prefix of a string addr
pub(crate) fn trim_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

// Convert a `SuiAddress` to `ObjectID` using its inner bytes
pub(crate) fn to_object_id(addr: SuiAddress) -> Result<ObjectID> {
    let bytes = addr.to_inner();
    ObjectID::from_bytes(bytes).map_err(|e| Error::TransportError(e.to_string()))
}

pub(crate) fn new_callarg<T: Serialize>(input: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(
        bcs::to_bytes(input)
            .map_err(|e| Error::TransportError(e.to_string()))?
    ))
}

pub(crate) fn add_input<T: Serialize>(ptb: &mut PTB, input: &T) -> Result<Argument> {
    let arg = ptb.input(new_callarg(input)?)
        .map_err(|e| Error::TransportError(e.to_string()))?;
    Ok(arg)
}

// The path of each struct or object (like a coin) on Sui consists of 3 parts:
// package_id, module, name. For example: "0x02::sui::SUI" is full path to SUI coin,
// "0xcafbc...::player::PlayerProfile" is the full path to struct "PlayerProfile"
type SuiPathParts = (String, String, String);
pub(crate) fn parse_sui_path(path: &str) -> Result<SuiPathParts > {
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() != 3 {
        return Err(Error::TransportError(format!("Invalid sui path {}", path)));
    }

    Ok((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
}

// Create a StructTag to match a specific struct (or object) defined in Sui move
// Param indicates a full object/struct path: `"package::module::struct"`.  Find
// constant paths in the `constants.rs` mod
pub(crate) fn new_structtag(path: &str) -> Result<StructTag> {
    let parts: SuiPathParts = parse_sui_path(path)?;
    Ok(StructTag {
        address: parse_account_addr(&parts.0)?,
        module: new_identifier(&parts.1)?,
        name: new_identifier(&parts.2)?,
        type_params: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sui_sdk::types::base_types::SuiAddress;

    #[test]
    fn test_add_inputs() -> Result<()> {
        let owner = RecipientSlotOwner::Unassigned { identifier: "Race1".into() };
        let (owner_type, owner_info) = match &owner {
            &RecipientSlotOwner::Unassigned { ref identifier } => (0u8, identifier.clone()),
            &RecipientSlotOwner::Assigned { ref addr } => (1u8, addr.to_string())
        };

        let move_owner_type = new_callarg(&owner_type)?;
        let move_owner_info = new_callarg(&owner_info)?;
        let move_weights = new_callarg(&10u16)?;

        assert_eq!(move_owner_type, CallArg::Pure(vec![0u8]));
        assert_eq!(move_owner_info, CallArg::Pure(vec![5, 82u8, 97u8, 99u8, 101u8, 49u8]));
        assert_eq!(move_weights, CallArg::Pure(vec![10, 0]));

        Ok(())
    }

    #[test]
    fn test_parse_coin_type() -> Result<()> {
        let (sui_addr, sui_module, sui_name) = parse_sui_path(COIN_SUI_ADDR)?;
        let (usdc_addr, usdc_module, usdc_name) = parse_sui_path(COIN_USDC_ADDR)?;

        assert_eq!(sui_addr, "0x2");
        assert_eq!(sui_module, "sui");
        assert_eq!(sui_name, "SUI");
        assert_eq!(usdc_addr, "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf");
        assert_eq!(usdc_module, "usdc");
        assert_eq!(usdc_name, "USDC");

        Ok(())
    }
}
