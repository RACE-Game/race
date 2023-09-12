use clap::{arg, Command};
use prettytable::{row, Table};
use race_core::{
    transport::TransportT,
    types::{
        CreateGameAccountParams, CreateRecipientParams, CreateRegistrationParams, EntryType,
        PublishGameParams, QueryMode, RecipientSlotInit, RegisterGameParams, ServerAccount,
        UnregisterGameParams,
    },
};
use race_env::{default_keyfile, parse_with_default_rpc};
use race_transport::TransportBuilder;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, sync::Arc};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecipientSpecs {
    Slots(Vec<RecipientSlotInit>),
    Addr(String),
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
        .subcommand(
            Command::new("publish")
                .about("Publish a game bundle")
                .arg(arg!(<NAME> "The name of game"))
                .arg(arg!(<BUNDLE> "The path to the WASM bundle"))
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
            Command::new("unreg-game")
                .about("Unregister game account")
                .arg(arg!(<REG> "The address of registration account"))
                .arg(arg!(<GAME> "The address of game account"))
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

async fn publish(name: String, bundle: String, transport: Arc<dyn TransportT>) {
    let params = PublishGameParams {
        uri: bundle,
        name,
        symbol: "RACEBUNDLE".into(),
    };
    let resp = transport.publish_game(params).await.expect("RPC error");
    println!("Address: {}", &resp);
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

async fn game_info(addr: &str, transport: Arc<dyn TransportT>) {
    let mode = QueryMode::default();
    match transport
        .get_game_account(addr, mode)
        .await
        .expect("Network error")
    {
        Some(game_account) => {
            println!("Game account: {}", game_account.addr);
            println!("Game bundle: {}", game_account.bundle_addr);
            println!("Access version: {}", game_account.access_version);
            println!("Settle version: {}", game_account.settle_version);
            println!("Data size: {}", game_account.data.len());
            println!("Max players: {}", game_account.max_players);
            println!("Entry type: {:?}", game_account.entry_type);
            println!("Recipient account: {}", game_account.recipient_addr);
            println!("Players:");
            for p in game_account.players.iter() {
                println!(
                    "Player[{}] position: {} @{}",
                    p.addr, p.position, p.access_version
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

async fn unreg_game(reg_addr: String, game_addr: String, transport: Arc<dyn TransportT>) {
    println!(
        "Unregister game {} from registration {}",
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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let matches = cli().get_matches();

    let chain = matches.get_one::<String>("chain").expect("required");
    let rpc = parse_with_default_rpc(chain, matches.get_one::<String>("rpc").expect("required"));
    let keyfile = matches.get_one::<String>("keyfile");

    println!("Interact with chain: {:?}", chain);
    println!("RPC Endpoint: {:?}", rpc);

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let bundle = sub_matches.get_one::<String>("BUNDLE").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            publish(name.to_owned(), bundle.to_owned(), transport).await;
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
        Some(("unreg-game", sub_matches)) => {
            let reg_addr = sub_matches.get_one::<String>("REG").expect("required");
            let game_addr = sub_matches.get_one::<String>("GAME").expect("required");
            let transport = create_transport(&chain, &rpc, keyfile.cloned()).await;
            unreg_game(reg_addr.clone(), game_addr.clone(), transport).await;
        }
        Some(("reg-info", sub_matches)) => {
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            let transport = create_transport(&chain, &rpc, None).await;
            reg_info(addr, transport).await;
        }
        _ => unreachable!(),
    }
}
