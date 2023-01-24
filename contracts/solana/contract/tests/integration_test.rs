use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use race_contract;
use race_core::types::{CreateGameAccountParams, GameAccount};
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

pub async fn create_game(
    mut client: BanksClient,
    program_id: Pubkey,
    user: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundle_addr = std::iter::repeat("X").take(10).collect::<String>();
    let params = CreateGameAccountParams {
        bundle_addr: bundle_addr.clone(),
        max_players: 5,
        // TODO: if make 10000 or higher - test will fail. FIXME
        data: std::iter::repeat(1).take(1000).collect::<Vec<u8>>(),
    };
    let (account_addr, _) = Pubkey::find_program_address(
        &[race_contract::instruction::game_account_seed(user.pubkey()).as_slice()],
        &program_id,
    );

    let update_profile_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(account_addr, false),
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(user.pubkey(), true),
        ],
        data: race_contract::instruction::RaceContractInstruction::CreateGame(params)
            .try_to_vec()?,
    };

    create_and_send_tx(
        client.clone(),
        vec![update_profile_instruction],
        vec![user],
        Some(&user.pubkey()),
    )
    .await?;

    let account_acc = client
        .get_account(account_addr)
        .await?
        .ok_or("could not find program state account")?;

    let account = GameAccount::deserialize(&mut account_acc.data.as_slice())?;
    // println!("Got game account {:?} ", updated_profile);
    assert_eq!(account.bundle_addr, bundle_addr);
    assert_eq!(account.addr, account_addr.to_string());
    Ok(())
}

// #[tokio::test(threaded_scheduler)]

#[tokio::test]
async fn test_race_contract() -> Result<(), Box<dyn Error>> {
    let program_id = race_contract::id();
    let pt = solana_program_test::ProgramTest::new(
        "mpl_program_test",
        program_id,
        solana_program_test::processor!(race_contract::entrypoint::process_instruction),
    );
    let (client, mint, _) = pt.start().await;

    let user = Keypair::new();
    request_airdrop(client.clone(), &mint, &user, LAMPORTS_PER_SOL * 10).await?;

    create_game(client, program_id, &user).await?;

    Ok(())
}
