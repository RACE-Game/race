use clap::{arg, Command};
use race_core::{
    transport::TransportT,
    types::{CreateRegistrationParams, GameBundle, ServerAccount},
};
use race_env::Config;
use race_transport::TransportBuilder;
use std::{fs::File, io::Read};
use base64::Engine;

fn cli() -> Command {
    Command::new("cli")
        .about("Command line tools for Race Protocol")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(
            Command::new("publish")
                .about("Publish a game bundle")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<BUNDLE> "The path to the WASM bundle"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("bundle-info")
                .about("Query game bundle information")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<ADDRESS> "The game bundle address"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("game-info")
                .about("Query game account information")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<ADDRESS> "The game account address"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("server-info")
                .about("Query server account information")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<ADDRESS> "The server account address"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("create-reg")
                .about("Create registration center")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("reg-info")
                .about("Query registration center")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<ADDRESS> "The address of registration account"))
                .arg_required_else_help(true),
        )
}

async fn create_transport(config: &Config, chain: &str) -> Box<dyn TransportT> {
    TransportBuilder::default()
        .try_with_chain(chain)
        .expect("Invalid chain")
        .try_with_config(config)
        .expect("Invalid config")
        .build()
        .await
        .expect("Failed to create transport")
}

async fn publish(config: Config, chain: &str, bundle: &str) {
    let transport: Box<dyn TransportT> = create_transport(&config, chain).await;
    let mut file = File::open(bundle).unwrap();
    let mut buf = Vec::with_capacity(0x4000);
    file.read_to_end(&mut buf).unwrap();
    let addr = "facade-program-addr".into();
    let base64 = base64::prelude::BASE64_STANDARD;
    let data = base64.encode(&buf);
    let bundle = GameBundle { addr, data };
    let resp = transport.publish_game(bundle).await.expect("RPC error");
    println!("Address: {:?}", &resp);
}

async fn bundle_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain).await;
    match transport.get_game_bundle(addr).await {
        Some(game_bundle) => {
            println!("Game bundle: {:?}", game_bundle.addr);
            println!("Data size: {:?}", game_bundle.data.len());
        }
        None => {
            println!("Game bundle not found");
        }
    }
}

async fn game_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain).await;
    match transport.get_game_account(addr).await {
        Some(game_account) => {
            println!("Game account: {}", game_account.addr);
            println!("Game bundle: {}", game_account.bundle_addr);
            println!("Access version: {}", game_account.access_version);
            println!("Settle version: {}", game_account.settle_version);
            println!("Data size: {}", game_account.data.len());
            println!("Players:");
            for p in game_account.players.iter() {
                println!("Player[{}] position: {}", p.addr, p.position);
            }
            println!("Deposits:");
            for d in game_account.deposits.iter() {
                println!("Deposit: from[{}], amount: {}", d.addr, d.amount);
            }
            println!("Servers:");
            for s in game_account.servers.iter() {
                println!("Server[{}]: {}", s.endpoint, s.addr);
            }
            println!("Current transactor: {:?}", game_account.transactor_addr);
        }
        None => {
            println!("Game bundle not found");
        }
    }
}

async fn server_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain).await;
    match transport.get_server_account(addr).await {
        Some(server_account) => {
            let ServerAccount {
                addr,
                owner_addr,
                endpoint,
            } = server_account;
            println!("Server account: {}", addr);
            println!("Server owner: {}", owner_addr);
            println!("Server owner: {}", endpoint);
        }
        None => {
            println!("Server not found");
        }
    }
}

async fn reg_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain).await;
    match transport.get_registration(addr).await {
        Some(reg) => {
            println!("Registration account: {}", reg.addr);
            println!("Size(Registered): {}({})", reg.size, reg.games.len());
            println!("Owner: {}", reg.owner.unwrap_or("None".into()));
            println!("Games:");
            for g in reg.games.iter() {
                println!(
                    "Game account: {}, Game bundle: {}",
                    g.addr, g.bundle_addr
                );
            }
        }
        None => {
            println!("Registration not found");
        }
    }
}

async fn create_reg(config: Config, chain: &str) {
    let transport = create_transport(&config, chain).await;
    let params = CreateRegistrationParams {
        is_private: false,
        size: 100,
    };
    transport
        .create_registration(params)
        .await
        .expect("Create registration falied");
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Config::from_path(&config_path.into()).await;

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let bundle = sub_matches.get_one::<String>("BUNDLE").expect("required");
            publish(config, chain, bundle).await;
        }
        Some(("bundle-info", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            bundle_info(config, chain, addr).await;
        }
        Some(("game-info", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            game_info(config, chain, addr).await;
        }
        Some(("server-info", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            server_info(config, chain, addr).await;
        }
        Some(("create-reg", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            create_reg(config, chain).await;
        }
        Some(("reg-info", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let addr = sub_matches.get_one::<String>("ADDRESS").expect("required");
            reg_info(config, chain, addr).await;
        }
        _ => unreachable!(),
    }
}
