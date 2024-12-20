// TODO: make imports more specific?
use super::*;

/// This trait will be used for deserializing various fields in the returnd object.
/// The implementations in this module-level crate focus on the common types such as
/// `u8`, `u64`, `String` as well as compound generic types such as `Option<T>`. For
/// the contract specific types, refer to the module or crate where they are defined.
///
/// # Example
///
/// ```
/// let verify_key: String = get_mv_value(fields, "verify_key")?;
/// let id: ObjectID = get_mv_value(fields, "id")?;
/// let bytes: Vec<u8> = get_mv_vec(fields, "data")?;
/// ```
pub(crate) trait TryFromSuiMoveValue: Sized {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self>;
}

// Because Sui has to maintain its SDK compatibility with the JavaScript world,
// it converts `u64` (BigInt) to `String` in the returned value and all other
// numbers to `u32`. Following impls handle this inconsistency
impl TryFromSuiMoveValue for u8 {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Number(num) => {
                u8::try_from(*num)
                    .map_err(|e| Error::TransportError(e.to_string()))
            }
            _ => Err(Error::TransportError("expected number value".into()))
        }
    }
}

impl TryFromSuiMoveValue for u16 {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Number(num) => {
                u16::try_from(*num)
                    .map_err(|e| Error::TransportError(e.to_string()))
            }
            _ => Err(Error::TransportError("expected number value".into()))
        }
    }
}

impl TryFromSuiMoveValue for u32 {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Number(num) => Ok(*num),
            _ => Err(Error::TransportError("expected u32 but got sth else".into()))
        }
    }
}

impl TryFromSuiMoveValue for u64 {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::String(s) => s.parse::<u64>()
                .map_err(|e| Error::TransportError(e.to_string())),
            _ => Err(Error::TransportError("expected string repr for u64".into()))
        }
    }
}

impl<T: TryFromSuiMoveValue> TryFromSuiMoveValue for Option<T> {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            // boxed_option: &Box<Option<SuiMoveValue>>
            SuiMoveValue::Option(boxed_option) => {
                match boxed_option.as_ref() {
                    None => Ok(None),
                    Some(inner) => Ok(Some(T::try_from_sui_move_value(inner)?))
                }
            },
            _ => Err(Error::TransportError("expected option value".into()))
        }
    }
}

impl TryFromSuiMoveValue for String {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::String(s) => Ok(s.clone()),
            _ => Err(Error::TransportError("expected string value".into()))
        }
    }
}

impl TryFromSuiMoveValue for ObjectID {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::UID {id} => Ok(*id),
            _ => Err(Error::TransportError("expected string value".into()))
        }
    }
}

impl TryFromSuiMoveValue for SuiAddress {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Address(addr) => Ok(*addr),
            _ => Err(Error::TransportError("expected string value".into()))
        }
    }
}

impl <T: TryFromSuiMoveValue> TryFromSuiMoveValue for Vec<T> {
    fn try_from_sui_move_value(value: &SuiMoveValue) -> Result<Self> {
        match value {
            SuiMoveValue::Vector(vals) => {
                vals.iter().map(|v| T::try_from_sui_move_value(v)).collect()
            },
            _ => Err(Error::TransportError("expected vector value".into()))
        }
    }
}

// Entry point function for get a specific move value
pub(crate) fn get_mv_value<T: TryFromSuiMoveValue>(
    fields: &BTreeMap<String, SuiMoveValue>,
    key: &str
) -> Result<T> {
    fields.get(key)
        .ok_or_else(|| Error::TransportError(format!("{} -> n/a", key)))
        .and_then(|value| T::try_from_sui_move_value(value))
}

// TODO: convert move get_mv_* to trait implementation for different types
// Slot related enums
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
    use sui_json_rpc_types::SuiMoveValue;

    // Helper function to create SuiMoveValue variants for testing
    fn create_mv_value<T: Into<SuiMoveValue>>(value: T) -> SuiMoveValue {
        value.into()
    }

    #[test]
    fn test_try_from_sui_move_value() -> Result<()> {
        // Test values
        let num_u8: u8 = 8;
        let num_u16: u16 = 16;
        let num_u32: u32 = 32;
        let num_u64: u64 = 64;
        let text: String = "test_string".into();
        let addr = SuiAddress::random_for_testing_only();
        let bytes: Vec<u8> = vec![1, 2, 3, 4, 5];

        // Create corresponding SuiMoveValues
        let mv_u8 = SuiMoveValue::Number(num_u8 as u32);
        let mv_u16 = SuiMoveValue::Number(num_u16 as u32);
        let mv_u32 = SuiMoveValue::Number(num_u32);
        // u64 is represented as string
        let mv_u64 = SuiMoveValue::String(num_u64.to_string());
        let mv_string = SuiMoveValue::String(text.clone());
        let mv_addr = SuiMoveValue::Address(addr);
        let mv_bytes = SuiMoveValue::Vector(
            bytes.iter()
                .map(|&b| SuiMoveValue::Number(b as u32))
                .collect()
        );

        // Test deserialization
        assert_eq!(u8::try_from_sui_move_value(&mv_u8)?, num_u8);
        assert_eq!(u16::try_from_sui_move_value(&mv_u16)?, num_u16);
        assert_eq!(u32::try_from_sui_move_value(&mv_u32)?, num_u32);
        assert_eq!(u64::try_from_sui_move_value(&mv_u64)?, num_u64);
        assert_eq!(String::try_from_sui_move_value(&mv_string)?, text);
        assert_eq!(SuiAddress::try_from_sui_move_value(&mv_addr)?, addr);
        assert_eq!(Vec::<u8>::try_from_sui_move_value(&mv_bytes)?, bytes);

        // Test error cases
        // wrong type
        assert!(u8::try_from_sui_move_value(&mv_string).is_err());
        // overflow
        assert!(u8::try_from_sui_move_value(&SuiMoveValue::Number(256)).is_err());
        // wrong type
        assert!(String::try_from_sui_move_value(&mv_u8).is_err());

        Ok(())
    }

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
