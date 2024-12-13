// Helper functions for interacting with Sui
use crate::error::{TransportError, TransportResult};
use race_core::error::{Error, Result};
use serde::Serialize;
use std::str::FromStr;
use sui_config::{sui_config_dir, SUI_KEYSTORE_FILENAME, SUI_CLIENT_CONFIG};
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::{
    rpc_types::SuiTransactionBlockResponseOptions,
    types::{
        base_types::{ObjectID, SuiAddress},
        crypto::{get_key_pair_from_rng, SuiKeyPair},
        programmable_transaction_builder::ProgrammableTransactionBuilder as PTB,
        transaction::{Argument, CallArg, Command, Transaction, TransactionData},
        quorum_driver_types::ExecuteTransactionRequestType,
        Identifier,
    },
    SuiClient, SuiClientBuilder,
    SUI_DEVNET_URL, SUI_COIN_TYPE,
};

pub(crate) fn new_callarg<T: Serialize>(input: &T) -> TransportResult<CallArg> {
    Ok(CallArg::Pure(
        bcs::to_bytes(&input)
            .map_err(|e| Error::ExternalError(
                format!("Failed to serialize due to Error: {}", e)
            ))?
    ))
}

pub(crate) fn new_identifier(literal: &str) -> TransportResult<Identifier> {
    Identifier::new(literal)
        .map_err(|_| TransportError::FailedToIdentify(literal.into()))
}

pub(crate) fn parse_str_addr(value: &str) -> TransportResult<SuiAddress> {
    SuiAddress::from_str(value)
        .map_err(|_| TransportError::ParseAddressError)
}

pub(crate) fn add_input<T: Serialize>(ptb: &mut PTB, input: &T) -> TransportResult<()> {
    ptb.input(new_callarg(input)?)
        .map_err(|_| Error::ExternalError("Failed to add ptb input".into()))?;
    Ok(())
}
