use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use race_contract;
use race_contract::entrypoint::UserAccount;
use race_core::types::{CloseGameAccountParams, CreateGameAccountParams, GameAccount};
use solana_client::rpc_client::RpcClient;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::program_error::ProgramError;
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
fn create_vault_account(client: &RpcClient, program_id: Pubkey, payer: &Keypair) {
    // Derive the PDA from the payer account, a string representing the unique
    // purpose of the account ("vault"), and the address of our on-chain program.
    let (vault_pubkey, vault_bump_seed) =
        Pubkey::find_program_address(&[b"vault", payer.pubkey().as_ref()], &program_id);

    // Get the amount of lamports needed to pay for the vault's rent
    let vault_account_size = usize::try_from(1000).unwrap();
    let lamports = client
        .get_minimum_balance_for_rent_exemption(vault_account_size)
        .unwrap();

    // The on-chain program's instruction data, imported from that program's crate.
    let instr_data = race_contract::entrypoint::InstructionData {
        vault_bump_seed,
        lamports,
    };

    // The accounts required by both our on-chain program and the system program's
    // `create_account` instruction, including the vault's address.
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(vault_pubkey, false),
        AccountMeta::new(system_program::ID, false),
    ];

    // Create the instruction by serializing our instruction data via borsh
    let instruction = Instruction::new_with_borsh(program_id, &instr_data, accounts);

    let blockhash = client.get_latest_blockhash().unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    client.send_and_confirm_transaction(&transaction).unwrap();
}
pub async fn send_instruction(
    client: BanksClient,
    program_id: Pubkey,
    instantiater: &Keypair,
    data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (vault_pubkey, vault_bump_seed) =
        Pubkey::find_program_address(&[b"vault", instantiater.pubkey().as_ref()], &program_id);
    println!("BUBUB {} {}", vault_pubkey, vault_bump_seed);
    // let lamports = 10000;
    // TODO need something like client.get_minimum_balance_for_rent_exemption(usize::try_from(VAULT_ACCOUNT_SIZE)?)? here

    let instantiate_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta {
                pubkey: instantiater.pubkey(),
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: vault_pubkey,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: system_program::id(),
                is_signer: false,
                is_writable: false,
            },
            // AccountMeta::new(instantiater.pubkey(), true),
            // AccountMeta::new(vault_pubkey, false),
            // AccountMeta::new(system_program::id(), false),
        ],
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
    // profile: Profile,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundle_addr = std::iter::repeat("X").take(10).collect::<String>();
    let params = CreateGameAccountParams {
        bundle_addr: bundle_addr.clone(),
        max_players: 5,
        // TODO: if make 10000 or higher - test will fail. FIXME
        data: std::iter::repeat(1).take(1000).collect::<Vec<u8>>(),
    };
    let (account_addr, _) =
        Pubkey::find_program_address(&[UserAccount::seed(user.pubkey()).as_slice()], &program_id);

    let update_profile_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(account_addr, false),
            AccountMeta::new(system_program::id(), false),
            AccountMeta::new(user.pubkey(), true),
        ],
        data: race_contract::entrypoint::Instruction::CreateGame(params).try_to_vec()?,
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
async fn tesfdft() -> Result<(), Box<dyn Error>> {
    // let program_id = Pubkey::new_unique();
    // let payer = Keypair::new();

    // let client = RpcClient::new("https://api.testnet.solana.com");

    // create_vault_account(&client, program_id, &payer);
    // return Ok(());

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
    // send_create_game_instruction(
    //     client.clone(),
    //     program_id,
    //     &user,
    //     CreateGameAccountParams {
    //         bundle_addr: "addr".to_string(),
    //         max_players: 5,
    //         data: vec![1, 2, 3],
    //     },
    // )
    // .await?;

    // send_close_game_instruction(
    //     client.clone(),
    //     program_id,
    //     &user,
    //     CloseGameAccountParams {
    //         addr: "addr".to_string(),
    //     },
    // )
    // .await?;
    Ok(())
}
