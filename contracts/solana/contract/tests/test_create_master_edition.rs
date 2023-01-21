use std::error::Error;

use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program_test::BanksClient;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::{
    program_pack::Pack,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};

// Copied from
// https://github.com/shravanshetty1/blawgd-solana/blob/908f80a69050feec7b6cd53631813b316f54e1cc/blawgd-solana-sc/tests/helper/mod.rs
pub async fn instantiate_program(
    mut client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let instantiate_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(instantiater.pubkey(), true),
        ],
        data: [].into(),
    };

    create_and_send_tx(
        client.clone(),
        vec![instantiate_instruction],
        vec![instantiater],
        Some(&instantiater.pubkey()),
    )
    .await?;

    Ok(())
}

pub async fn request_airdrop(
    client: BanksClient,
    mint: &Keypair,
    user: &Keypair,
    lamports: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let ix = system_instruction::transfer(&mint.pubkey(), &user.pubkey(), lamports);
    create_and_send_tx(client, vec![ix], vec![mint], Some(&mint.pubkey())).await?;
    Ok(())
}

pub async fn create_and_send_tx(
    mut client: BanksClient,
    instructions: Vec<Instruction>,
    signers: Vec<&dyn Signer>,
    payer: Option<&Pubkey>,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = Message::new(instructions.as_slice(), payer);
    let tx = Transaction::new(&signers, msg, client.get_latest_blockhash().await?);
    Ok(client.process_transaction(tx).await?)
}

#[cfg(feature = "test-bpf")]
#[tokio::test]
async fn test_create_master_edition() -> Result<(), Box<dyn Error>> {
    let program_id = mpl_program_test::id();
    let pt = solana_program_test::ProgramTest::new(
        "mpl_program_test",
        program_id,
        solana_program_test::processor!(mpl_program_test::entrypoint::process_instruction),
    );
    let (client, mint, _) = pt.start().await;

    let user = Keypair::new();
    request_airdrop(client.clone(), &mint, &user, LAMPORTS_PER_SOL * 10).await?;

    instantiate_program(client.clone(), program_id, &user).await?;
    println!("instantiated smart contract");

    if instantiate_program(client.clone(), program_id, &user)
        .await
        .is_err()
    {
        println!("success - failed to instantiate smart contract twice");
    } else {
        println!("failed - instantiated smart contract twice");
    }
    Ok(())
}
