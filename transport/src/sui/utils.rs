// Helper functions for interacting with Sui
// TODO: make imports more specific?
use super::*;

pub(crate) fn new_identifier(literal: &str) -> TransportResult<Identifier> {
    Identifier::new(literal)
        .map_err(|_| TransportError::FailedToIdentify(literal.into()))
}

pub(crate) fn parse_str_addr(value: &str) -> TransportResult<SuiAddress> {
    SuiAddress::from_str(value)
        .map_err(|_| TransportError::ParseAddressError)
}

pub(crate) fn new_callarg<T: Serialize>(input: &T) -> TransportResult<CallArg> {
    Ok(CallArg::Pure(
        bcs::to_bytes(input)
            .map_err(|e| Error::ExternalError(
                format!("Failed to serialize due to Error: {}", e)
            ))?
    ))
}

pub(crate) fn add_input<T: Serialize>(ptb: &mut PTB, input: &T) -> TransportResult<Argument> {
    let arg = ptb.input(new_callarg(input)?)
        .map_err(|_| Error::ExternalError("Failed to add ptb input".into()))?;
    Ok(arg)
}

// coin has 3 types of info: coin account address, coin module, coin name
type CoinInfo = (String, String, String);
pub(crate) fn parse_coin_type(coin_str: &str) -> TransportResult<CoinInfo> {
    let parts: Vec<&str> = coin_str.split("::").collect();
    if parts.len() != 3 {
        return Err(TransportError::InvalidCoinType(coin_str.to_string()));
    }

    Ok((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_inputs() -> TransportResult<()> {
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
    fn test_parse_coin_type() -> TransportResult<()> {
        let sui = "0x2::sui::SUI";
        let usdc = "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::usdc::USDC";

        let (sui_addr, sui_module, sui_name) = parse_coin_type(sui)?;
        let (usdc_addr, usdc_module, usdc_name) = parse_coin_type(usdc)?;

        assert_eq!(sui_addr, "0x2");
        assert_eq!(sui_module, "sui");
        assert_eq!(sui_name, "SUI");
        assert_eq!(usdc_addr, "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf");
        assert_eq!(usdc_module, "usdc");
        assert_eq!(usdc_name, "USDC");

        Ok(())
    }
}
