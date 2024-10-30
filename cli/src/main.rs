use clap::{arg, Command};
use prettytable::{row, Table};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreateRecipientParams,
        CreateRegistrationParams, EntryType, PublishGameParams, QueryMode, RecipientClaimParams,
        RecipientSlotInit, RegisterGameParams, ServerAccount, UnregisterGameParams,
    },
};
use race_env::{default_keyfile, parse_with_default_rpc};
use race_storage::{
    arweave::Arweave,
    metadata::{make_metadata, MetadataT},
};
use race_transport::TransportBuilder;
use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;
use std::{
    fs::{self, File}, path::PathBuf, sync::Arc
};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecipientSpecs {
    Slots(Vec<RecipientSlotInit>),
    Addr(String),
}

impl RecipientSpecs {
    pub fn from_file(path: PathBuf) -> Self {
        let f = File::open(path).expect("Spec file not found");
        serde_json::from_reader(f).expect("Invalid spec file")
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameSpecs {
    title: String,
    reg_addr: String,
    token_addr: String,
    bundle_addr: String,
    max_players: u16,
    entry_type: EntryType,
    recipient: RecipientSpecs,
    data: Vec<u8>,
}

impl CreateGameSpecs {
    pub fn from_file(path: PathBuf) -> Self {
        let f = File::open(path).expect("Spec file not found");
        serde_json::from_reader(f).expect("Invalid spec file")
    }
}

fn cli() -> Command {
    Command::new("cli")
        .about("Command line tools for Race Protocol")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <chain> "The chain to interact").required(true))
        .arg(arg!(-r <rpc> "The endpoint of RPC service").required(true))
        .arg(arg!(-k <keyfile> "The path to keyfile"))
        .arg(arg!(-a <arweave_keyfile> "The path to Arweave JWK keyfile"))
        .subcommand(
            Command::new("publish")
                .about("Publish a game bundle")
                .arg(arg!(<NAME> "The name of game"))
                .arg(arg!(<SYMBOL> "The symbol used for game metadata file"))
                .arg(arg!(<CREATOR> "The creator address"))
                .arg(arg!(<BUNDLE> "The file path to game bundle"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("mint-nft")
                .about("Mint NFT with an Arweave URL")
                .arg(arg!(<NAME> "The name of game"))
                .arg(arg!(<SYMBOL> "The symbol used for game metadata file"))
                .arg(arg!(<ARWEAVE_URL> "The Arweave URL"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("bundle-info")
                .about("Query game bundle information")
                .arg(arg!(<ADDRESS> "The game bundle address"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("game-info")
                .about("Query game account information")
                .arg(arg!(<ADDRESS> "The game account address"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("server-info")
                .about("Query server account information")
                .arg(arg!(<ADDRESS> "The server account address"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("create-reg").about("Create registration center"))
        .subcommand(
            Command::new("reg-info")
                .about("Query registration center")
                .arg(arg!(<ADDRESS> "The address of registration account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("create-game")
                .about("Create game account")
                .arg(arg!(<SPEC_FILE> "The path to specification file"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("reg-game")
                .about("Register game account")
                .arg(arg!(<REG> "The address of registration account"))
                .arg(arg!(<GAME> "The address of game account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("close-game")
                .about("Close game account")
                .arg(arg!(<GAME> "The address of game account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("close-all-games")
                .about("Unregister and close all games for a registration")
                .arg(arg!(<REG> "The address of registration account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("unreg-game")
                .about("Unregister game account")
                .arg(arg!(<REG> "The address of registration account"))
                .arg(arg!(<GAME> "The address of game account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("create-recipient")
                .about("Create recipient account")
                .arg(arg!(<SPEC_FILE> "The path to specification file"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("recipient-info")
                .about("Query recipient account")
                .arg(arg!(<ADDRESS> "The address of recipient account"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("claim")
                .about("Claim tokens from a recipient account")
                .arg(arg!(<ADDRESS> "The address of recipient account"))
                .arg_required_else_help(true),
        )
}

async fn create_transport(chain: &str, rpc: &str, keyfile: Option<String>) -> Arc<dyn TransportT> {
    let mut builder = TransportBuilder::default()
        .try_with_chain(chain)
        .expect("Invalid chain")
        .with_rpc(rpc);

    if let Some(keyfile) = keyfile.or(default_keyfile(chain)) {
        println!("Use keyfile: {}", keyfile);
        builder = builder.with_keyfile(keyfile);
    }

    let transport = builder.build().await.expect("Failed to create transport");
    Arc::from(transport)
}

async fn mint_nft(
    name: String,
    symbol: String,
    arweave_url: String,
    transport: Arc<dyn TransportT>,
) {
    let params = PublishGameParams {
        uri: arweave_url,
        name,
        symbol,
    };
    let resp = transport.publish_game(params).await.expect("RPC error");
    println!("Address: {}", &resp);
}

async fn publish(
    chain: &str,
    name: String,
    symbol: String,
    creator_addr: String,
    bundle: String,
    arkey_path: String,
    transport: Arc<dyn TransportT>,
) {
    let mut arweave = Arweave::try_new(&arkey_path).expect("Creating arweave failed");
    let data = fs::read(PathBuf::from(&bundle)).expect("Wasm bundle not found");
    let bundle_addr = arweave
        .upload_file(data, None)
        .await
        .expect("Arweave uploading wasm bundle failed");
    let metadata = make_metadata(
        chain,
        name.clone(),
        symbol.clone(),
        creator_addr,
        bundle_addr.clone(),
    )
    .expect("Creating metadata failed");
    let json_meta = metadata.json_vec().expect("Jsonify metadata failed");
    let meta_addr = arweave
        .upload_file(json_meta, Some("application/json"))
        .await
        .expect("Arweave uploading metadata failed");

    let params = PublishGameParams {
        uri: meta_addr,
        name,
        symbol,
    };
    let resp = transport.publish_game(params).await.expect("RPC error");
    println!("Wasm bundle: {}", bundle_addr);
    println!("Address: {}", &resp);
}

async fn claim(addr: &str, transport: Arc<dyn TransportT>) {
    let params = RecipientClaimParams {
        recipient_addr: addr.into(),
    };

    transport
        .recipient_claim(params)
        .await
        .expect("Failed to claim tokens");
    println!("Done");
}

async fn bundle_info(addr: &str, transport: Arc<dyn TransportT>) {
    match transport
        .get_game_bundle(addr)
        .await
        .expect("Network error")
    {
        Some(game_bundle) => {
            println!("Game bundle: {:?}", game_bundle.name);
        }
        None => {
            println!("Game bundle not found");
        }
    }
}

#[allow(unused)]
fn print_hex(data: Vec<u8>) {
    let mut row = vec![];
    for i in data {
        row.push(format!("{:02x}", i));
    }
    let rows = row
        .chunks(8)
        .map(|rows| rows.join(" "))
        .collect::<Vec<String>>();
    for row in rows {
        println!("{}", row)
    }
}

async fn game_info(addr: &str, transport: Arc<dyn TransportT>) {
    let mode = QueryMode::default();
    match transport
        .get_game_account(addr, mode)
        .await
        .expect("Network error")
    {
        Some(game_account) => {
            println!("Game account: {}", game_account.addr);
            println!("Title: {}", game_account.title);
            println!("Game bundle: {}", game_account.bundle_addr);
            println!("Token address: {}", game_account.token_addr);
            println!("Access version: {}", game_account.access_version);
            println!("Settle version: {}", game_account.settle_version);
            println!("Data size: {}", game_account.data.len());
            println!("Max players: {}", game_account.max_players);
            println!("Entry type: {:?}", game_account.entry_type);
            println!("Recipient account: {}", game_account.recipient_addr);
            println!("Players:");
            for p in game_account.players.iter() {
                println!(
                    "Player[{}] position: {}, amount: {}, @{}",
                    p.addr, p.position, p.balance, p.access_version
                );
            }
            println!("Deposits:");
            for d in game_account.deposits.iter() {
                println!("Deposit: from[{}], amount: {}", d.addr, d.amount);
            }
            println!("Servers:");
            for s in game_account.servers.iter() {
                println!("Server[{}]: {} @{}", s.endpoint, s.addr, s.access_version);
            }
            println!("Votes:");
            for v in game_account.votes.iter() {
                println!("Vote from {} to {} for {:?}", v.voter, v.votee, v.vote_type);
            }
            println!("Current transactor: {:?}", game_account.transactor_addr);
            if let Some(cp) = game_account.checkpoint_on_chain.as_ref() {
                println!("Checkpoint");
                println!("  Access Version: {}", cp.access_version);
                let root: String = cp.root.iter().map(|b| format!("{:02x}", b)).collect();
                println!("  Root: {}", root);
            } else {
                println!("Checkpoint: None");
            }
        }
        None => {
            println!("Game account not found");
        }
    }
}

async fn server_info(addr: &str, transport: Arc<dyn TransportT>) {
    match transport
        .get_server_account(addr)
        .await
        .expect("Network error")
    {
        Some(server_account) => {
            let ServerAccount { addr, endpoint } = server_account;
            println!("Server account: {}", addr);
            println!("Server endpoint: {}", endpoint);
        }
        None => {
            println!("Server not found");
        }
    }
}

async fn reg_info(addr: &str, transport: Arc<dyn TransportT>) {
    match transport
        .get_registration(addr)
        .await
        .expect("Network error")
    {
        Some(reg) => {
            println!("Registration account: {}", reg.addr);
            println!("Size(Registered): {}({})", reg.size, reg.games.len());
            println!("Owner: {}", reg.owner.unwrap_or("None".into()));
            let mut table = Table::new();
            table.add_row(row!["Title", "Address", "Bundle"]);
            for g in reg.games.iter() {
                table.add_row(row![g.title, g.addr, g.bundle_addr]);
            }
            table.printstd();
        }
        None => {
            println!("Registration not found");
        }
    }
}

async fn recipient_info(addr: &str, transport: Arc<dyn TransportT>) {
    match transport.get_recipient(addr).await.expect("Network error") {
        Some(recipient) => {
            println!("Recipient account: {}", recipient.addr);
            println!("Capcity account: {:?}", recipient.cap_addr);
            println!("Slots");
            for slot in recipient.slots.iter() {
                println!("|- id: {}", slot.id);
                println!("   type: {:?}", slot.slot_type);
                println!("   token: {}", slot.token_addr);
                println!("   balance: {}", slot.balance);
                println!("   Shares");
                for share in slot.shares.iter() {
                    println!("   |- owner: {:?}", share.owner);
                    println!("      weights: {}", share.weights);
                    println!("      claim amount: {}", share.claim_amount);
                }
            }
        }
        None => {
            println!("Recipient not found")
        }
    }
}

async fn create_reg(transport: Arc<dyn TransportT>) {
    let params = CreateRegistrationParams {
        is_private: false,
        size: 100,
    };
    let addr = transport
        .create_registration(params)
        .await
        .expect("Create registration failed");
    println!("Address: {}", addr);
}

async fn create_recipient(specs: RecipientSpecs, transport: Arc<dyn TransportT>) {
    match specs {
        RecipientSpecs::Slots(slots) => {
            let params = CreateRecipientParams {
                cap_addr: None,
                slots,
            };
            let addr = transport
                .create_recipient(params)
                .await
                .expect("Create recipient failed");
            println!("Recipient account created: {}", addr);
        }
        RecipientSpecs::Addr(_) => {
            println!("Invalid spec format");
        }
    };
}

async fn create_game(specs: CreateGameSpecs, transport: Arc<dyn TransportT>) {
    // println!("Specs: {:?}", specs);

    let CreateGameSpecs {
        title,
        reg_addr,
        token_addr,
        bundle_addr,
        max_players,
        entry_type,
        recipient,
        data,
    } = specs;

    let recipient_addr = match recipient {
        RecipientSpecs::Slots(slots) => {
            let params = CreateRecipientParams {
                cap_addr: None,
                slots,
            };
            let addr = transport
                .create_recipient(params)
                .await
                .expect("Create recipient failed");
            println!("Recipient account created: {}", addr);
            addr
        }
        RecipientSpecs::Addr(addr) => addr,
    };

    let params = CreateGameAccountParams {
        title,
        bundle_addr,
        token_addr,
        max_players,
        entry_type,
        recipient_addr: recipient_addr.clone(),
        data,
    };

    let addr = transport
        .create_game_account(params)
        .await
        .expect("Create game account failed");

    println!("Game account created: {}", addr);

    transport
        .register_game(RegisterGameParams {
            game_addr: addr.clone(),
            reg_addr,
        })
        .await
        .expect("Failed to register game");

    println!("Game registered");
    println!("Recipient account: {}", recipient_addr);
    println!("Game account: {}", addr);
}

async fn close_game(game_addr: String, transport: Arc<dyn TransportT>) {
    let r = transport
        .close_game_account(CloseGameAccountParams {
            addr: game_addr.to_owned(),
        })
        .await;
    if let Err(e) = r {
        println!("Failed to close game due to: {}", e.to_string());
    } else {
        println!("Game closed");
    }
}

async fn reg_game(reg_addr: String, game_addr: String, transport: Arc<dyn TransportT>) {
    println!("Register game {} from registration {}", game_addr, reg_addr);
    let r = transport
        .register_game(RegisterGameParams {
            game_addr: game_addr.to_owned(),
            reg_addr: reg_addr.to_owned(),
        })
        .await;
    if let Err(e) = r {
        println!("Failed to register game due to: {}", e.to_string());
    } else {
        println!("Game registered");
    }
}

async fn close_all_games(reg_addr: String, transport: Arc<dyn TransportT>) {
    println!(
        "Unregister and close all games from registration {}",
        reg_addr
    );
    let reg = transport
        .get_registration(&reg_addr)
        .await
        .expect("Failed to load registration account");
    match reg {
        Some(reg) => {
            for g in reg.games {
                unreg_game(reg_addr.clone(), g.addr, transport.clone()).await;
            }
        }
        None => {
            println!("Registration account {} not found", reg_addr);
        }
    }
}

async fn unreg_game(reg_addr: String, game_addr: String, transport: Arc<dyn TransportT>) {
    println!(
        "Unregister and close game {} from registration {}",
        game_addr, reg_addr
    );
    let r = transport
        .unregister_game(UnregisterGameParams {
            game_addr: game_addr.to_owned(),
            reg_addr: reg_addr.to_owned(),
        })
        .await;
    if let Err(e) = r {
        println!("Failed to unregister game due to: {}", e.to_string());
    } else {
        println!("Game unregistered");
    }
    close_game(game_addr, transport).await;
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .try_from_env()
                .expect("Failed to parse EnvFilter"),
        );
    tracing::subscriber::set_global_default(subscriber).expect("Failed to configure logger");

    let matches = cli().get_matches();

    let chain = matches.get_one::<String>("chain").expect("required");
    let rpc = parse_with_default_rpc(chain, matches.get_one::<String>("rpc").expect("required"));
    let keyfile = matches.get_one::<String>("keyfile");
    let arweave_keyfile = matches.get_one::<String>("arweave_keyfile");

    println!("Interact with chain: {:?}", chain);
    println!("RPC Endpoint: {:?}", rpc);
    println!("Specified keyfile: {:?}", keyfile);

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let symbol = sub_matches.get_one::<String>("SYMBOL").expect("required");
            let creator = sub_matches.get_one::<String>("CREATOR").expect("required");
            let bundle = sub_matches.get_one::<String>("BUNDLE").expect("required");

            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            publish(
                &chain,
                name.to_owned(),
                symbol.to_owned(),
                creator.to_owned(),
                bundle.to_owned(),
                arweave_keyfile
                    .expect("Arweave keyfile is required")
                    .to_owned(),
                transport,
            )
            .await;
        }
        Some(("mint-nft", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let symbol = sub_matches.get_one::<String>("SYMBOL").expect("required");
            let arweave_url = sub_matches
                .get_one::<String>("ARWEAVE_URL")
                .expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            mint_nft(
                name.to_owned(),
                symbol.to_owned(),
                arweave_url.to_owned(),
                transport,
            )
            .await;
        }
        Some(("bundle-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            bundle_info(addr, transport).await;
        }
        Some(("game-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            game_info(addr, transport).await;
        }
        Some(("server-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            server_info(addr, transport).await;
        }
        Some(("create-reg", _sub_matches)) => {
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            create_reg(transport).await;
        }
        Some(("create-game", sub_matches)) => {
            let spec_file = sub_matches
                .get_one::<String>("SPEC_FILE")
                .expect("required");
            let specs = CreateGameSpecs::from_file(spec_file.into());
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            create_game(specs, transport).await;
        }
        Some(("reg-game", sub_matches)) => {
            let reg_addr = sub_matches.get_one::<String>("REG").expect("required");
            let game_addr = sub_matches.get_one::<String>("GAME").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            reg_game(reg_addr.clone(), game_addr.clone(), transport).await;
        }
        Some(("unreg-game", sub_matches)) => {
            let reg_addr = sub_matches.get_one::<String>("REG").expect("required");
            let game_addr = sub_matches.get_one::<String>("GAME").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            unreg_game(reg_addr.clone(), game_addr.clone(), transport).await;
        }
        Some(("close-all-games", sub_matches)) => {
            let reg_addr = sub_matches.get_one::<String>("REG").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            close_all_games(reg_addr.clone(), transport).await;
        }
        Some(("close-game", sub_matches)) => {
            let game_addr = sub_matches.get_one::<String>("GAME").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            close_game(game_addr.clone(), transport).await;
        }
        Some(("reg-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            reg_info(addr, transport).await;
        }
        Some(("create-recipient", sub_matches)) => {
            let spec_file = sub_matches
                .get_one::<String>("SPEC_FILE")
                .expect("required");
            let specs = RecipientSpecs::from_file(spec_file.into());
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            create_recipient(specs, transport).await;
        }
        Some(("recipient-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            recipient_info(addr, transport).await;
        }
        Some(("claim", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            claim(addr, transport).await;
        }
        _ => unreachable!(),
    }
}
