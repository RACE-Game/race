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
    let hash = client.get_latest_blockhash().await?;
    let tx = Transaction::new(&signers, msg, hash);

    Ok(client.process_transaction(tx).await?)
}

pub async fn create_game(
    mut client: BanksClient,
    program_id: Pubkey,
    user: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundle_addr = std::iter::repeat("X").take(10).collect::<String>();
    let params = CreateGameAccountParams {
        title: "SOME_TITTLE".to_string(),
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
async fn create_account(
    mut client: BanksClient,
    user: &Keypair,
) -> Result<Keypair, Box<dyn Error>> {
    let temp_user_account = Keypair::new();
    println!("TEMP_USER {}", temp_user_account.pubkey().to_string());

    let instruction = solana_sdk::system_instruction::create_account(
        &user.pubkey(),
        &temp_user_account.pubkey(),
        1000000,
        100 as u64,
        &user.pubkey(),
    );

    let transfer = solana_sdk::system_instruction::transfer(
        &user.pubkey(),
        &temp_user_account.pubkey(),
        100000000,
    );

    create_and_send_tx(
        client.clone(),
        vec![instruction, transfer],
        vec![user, &temp_user_account],
        Some(&user.pubkey()),
    )
    .await?;
    Ok(temp_user_account)
}
#[tokio::test]
async fn test_race_contract() -> Result<(), Box<dyn Error>> {
    let program_id = race_contract::id();
    let mut pt = solana_program_test::ProgramTest::default();
    pt.add_program(
        "mpl_program_test",
        program_id,
        solana_program_test::processor!(race_contract::entrypoint::process_instruction),
    );

    let (client, mint, _) = pt.start().await;
    let user = Keypair::new();
    println!("USER {}", user.pubkey().to_string());
    request_airdrop(client.clone(), &mint, &user, LAMPORTS_PER_SOL * 10).await?;

    // create_game(client.clone(), program_id, &user).await?;

    let temp_user_account1 = create_account(client.clone(), &user).await?;
    let temp_user_account2 = create_account(client.clone(), &user).await?;
    let params = race_contract::instruction::JoinGameParams { amount: 10 };
    let id = spl_token::id();
    println!("TOKEND ID {}", id);
    println!(
        "TOKEND ID {}",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()
    );
    println!("TOKEND ID1 {}", temp_user_account1.pubkey().to_string());
    println!("TOKEND ID2 {}", temp_user_account2.pubkey().to_string());
    println!("TOKEND ID3 {}", user.pubkey().to_string());
    println!(
        "TOKEND ID3 {}",
        solana_program::incinerator::check_id(&temp_user_account2.pubkey())
    );

    // Some strange InvalidAccountData here
    // https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/processor.rs#L691
    let close_ix = spl_token::instruction::close_account(
        &id,
        &temp_user_account1.pubkey(),
        &temp_user_account2.pubkey(),
        &user.pubkey(),
        &[],
    )
    .unwrap();

    create_and_send_tx(
        client.clone(),
        vec![close_ix],
        vec![&user],
        Some(&user.pubkey()),
    )
    .await?;

    // let tx = Transaction::new_signed_with_payer(
    //     &[spl_token::instruction::close_account(
    //         &id,
    //         &temp_user_account1.pubkey(),
    //         &temp_user_account2.pubkey(),
    //         &user.pubkey(),
    //         &[],
    //     )
    //     .unwrap()],
    //     Some(&context.payer.pubkey()),
    //     &[&context.payer, &owner, &token_account],
    //     client.get_latest_blockhash(),
    // );
    // context.banks_client.process_transaction(tx).await.unwrap();
    // let update_profile_instruction = Instruction {
    //     program_id,
    //     accounts: vec![
    //         AccountMeta::new(user.pubkey(), true),
    //         AccountMeta::new(temp_user_account1.pubkey(), false),
    //         AccountMeta::new(temp_user_account2.pubkey(), false),
    //         AccountMeta::new(id, false),
    //     ],
    //     data: race_contract::instruction::RaceContractInstruction::JoinGame(params).try_to_vec()?,
    // };
    // create_and_send_tx(
    //     client.clone(),
    //     vec![update_profile_instruction],
    //     vec![&user],
    //     Some(&user.pubkey()),
    // )
    // .await?;
    Ok(())
}
