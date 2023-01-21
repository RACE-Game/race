use borsh::BorshSerialize;
use race_contract;
use race_core::types::{CloseGameAccountParams, CreateGameAccountParams};
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program_test::tokio;
use solana_program_test::BanksClient;
use solana_sdk::{
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use std::error::Error;

pub async fn send_instruction(
    client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
    data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let instantiate_instruction = Instruction {
        program_id,
        accounts: vec![AccountMeta::new(instantiater.pubkey(), true)],
        data: data,
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
pub async fn send_create_game_instruction(
    client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
    params: CreateGameAccountParams,
) -> Result<(), Box<dyn std::error::Error>> {
    send_instruction(
        client,
        program_id,
        instantiater,
        race_contract::entrypoint::Instruction::CreateGame(params).try_to_vec()?,
    )
    .await?;

    Ok(())
}

pub async fn send_close_game_instruction(
    client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
    params: CloseGameAccountParams,
) -> Result<(), Box<dyn std::error::Error>> {
    send_instruction(
        client,
        program_id,
        instantiater,
        race_contract::entrypoint::Instruction::CloseGame(params).try_to_vec()?,
    )
    .await?;

    Ok(())
}

// Copied from
// https://github.com/shravanshetty1/blawgd-solana/blob/908f80a69050feec7b6cd53631813b316f54e1cc/blawgd-solana-sc/tests/helper/mod.rs
pub async fn instantiate_program(
    client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let instantiate_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(instantiater.pubkey(), true),
        ],
        data: race_contract::entrypoint::Instruction::CreateGame(CreateGameAccountParams {
            bundle_addr: "addr".to_string(),
            max_players: 5,
            data: vec![1, 2, 3],
        })
        .try_to_vec()?,
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
async fn test() -> Result<(), Box<dyn Error>> {
    let program_id = race_contract::id();
    let pt = solana_program_test::ProgramTest::new(
        "mpl_program_test",
        program_id,
        solana_program_test::processor!(race_contract::entrypoint::process_instruction),
    );
    let (client, mint, _) = pt.start().await;

    let user = Keypair::new();
    request_airdrop(client.clone(), &mint, &user, LAMPORTS_PER_SOL * 10).await?;

    send_create_game_instruction(
        client.clone(),
        program_id,
        &user,
        CreateGameAccountParams {
            bundle_addr: "addr".to_string(),
            max_players: 5,
            data: vec![1, 2, 3],
        },
    )
    .await?;

    send_close_game_instruction(
        client.clone(),
        program_id,
        &user,
        CloseGameAccountParams {
            addr: "addr".to_string(),
        },
    )
    .await?;
    Ok(())
}
