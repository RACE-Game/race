use mpl_token_metadata::instruction::{create_master_edition_v3, create_metadata_accounts_v3};
use race_solana_types::types::PublishParams;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_program,
};
use spl_token::{
    instruction::{mint_to, set_authority, AuthorityType},
    state::{Account, Mint},
};

#[inline(never)]
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: PublishParams,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let payer = next_account_info(accounts_iter)?;

    let mint_account = next_account_info(accounts_iter)?;

    let token_account = next_account_info(accounts_iter)?;

    let ata_account = next_account_info(accounts_iter)?;

    let metadata_pda = next_account_info(accounts_iter)?;

    let edition_pda = next_account_info(accounts_iter)?;

    let token_program = next_account_info(accounts_iter)?;

    let metaplex_program = next_account_info(accounts_iter)?;

    let sys_rent = next_account_info(accounts_iter)?;

    let system_program = next_account_info(accounts_iter)?;

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // TODO: lot of necessary checkes
    let mint_state = Mint::unpack_unchecked(&mint_account.data.borrow())?;
    if !mint_state.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    //
    // if !token_account.is_initialized {
    //     return Err(ProgramError::UninitializedAccount);
    // }

    // Mint 1 token to token account
    msg!("Token mint: {}", mint_account.key);
    msg!("Minting 1 token to account: {}", token_account.key);
    invoke(
        &mint_to(
            &token_program.key,
            &mint_account.key,
            &token_account.key,
            &payer.key,
            &[&payer.key],
            1,
        )?,
        &[
            mint_account.clone(),
            payer.clone(),
            token_account.clone(),
            token_program.clone(),
            sys_rent.clone(),
        ],
    )?;

    // TODO: Creator
    let creator = vec![
        mpl_token_metadata::state::Creator {
            address: payer.key.clone(),
            verified: false,
            share: 100,
        },
        mpl_token_metadata::state::Creator {
            address: mint_account.key.clone(),
            verified: false,
            share: 0,
        },
    ];
    // Create metadata account
    msg!("Creating metadata account: {}", metadata_pda.key);
    let create_metadata_account_ix = create_metadata_accounts_v3(
        metaplex_program.key.clone(),
        metadata_pda.key.clone(),
        mint_account.key.clone(),
        payer.key.clone(),
        payer.key.clone(),
        payer.key.clone(),
        params.name,   // name
        params.symbol, // symbol
        params.uri,
        Some(creator), // creator
        1,             // fee basis point
        true,          // update authority to signer
        false,         // is mutable?
        None,          // optional collection
        None,          // optional use
        None,          // optional collection detail
    );

    invoke(
        &create_metadata_account_ix,
        &[
            metadata_pda.clone(),
            mint_account.clone(),
            payer.clone(),
            payer.clone(),
            metaplex_program.clone(),
            token_program.clone(),
            system_program.clone(),
            sys_rent.clone(),
        ],
    )?;

    // Create master edition account
    // mint_authority and freeze_authority will be transfer to this account
    msg!("Creating master edition account: {}", edition_pda.key);
    let create_master_edition_account_ix = create_master_edition_v3(
        metaplex_program.key.clone(),
        edition_pda.key.clone(),
        mint_account.key.clone(),
        payer.key.clone(), // update authority
        payer.key.clone(), // mint authority
        metadata_pda.key.clone(),
        payer.key.clone(),
        Some(0), // max_supply because token has been minted once
    );

    invoke(
        &create_master_edition_account_ix,
        &[
            edition_pda.clone(),
            mint_account.clone(),
            payer.clone(),
            payer.clone(),
            metadata_pda.clone(),
            metaplex_program.clone(),
            token_program.clone(),
            system_program.clone(),
            sys_rent.clone(),
        ],
    )?;

    msg!("Minted NFT successfully");

    Ok(())
}
