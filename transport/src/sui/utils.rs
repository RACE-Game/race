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

// TODO: replace with Sui transport's own RecipientXXX structs
pub(crate) fn add_share_inputs(ptb: &mut PTB, share: &RecipientSlotShareInit) -> TransportResult<()> {
    let (owner_type, owner_info) = match &share.owner {
        &RecipientSlotOwner::Unassigned { ref identifier } => (0u8, identifier.clone()),
        &RecipientSlotOwner::Assigned { ref addr } => (1u8, addr.to_string()),
    };
    add_input(ptb, &owner_type)?;
    add_input(ptb, &owner_info)?;
    add_input(ptb, &share.weights)?;
    Ok(())
}

// For each slot, the input indexes go as follows:
// Input(0): slot id
// Input(1): slot token addr
// Input(2): slot type
// Input(3 ... x): for slot shares
pub(crate) fn add_slot_inputs(ptb: &mut PTB, slot: &RecipientSlotInit) -> TransportResult<()> {
    add_input(ptb, &slot.id)?;
    let token_addr = parse_str_addr(&slot.token_addr)?;
    add_input(ptb, &token_addr)?;
    let slot_type = match slot.slot_type {
        RecipientSlotType::Nft => 0u8,
        RecipientSlotType::Token => 1u8,
    };
    add_input(ptb, &slot_type)?;

    for share in &slot.init_shares {
        add_share_inputs(ptb, share)?;
    }
    Ok(())
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
}
