use clap::{arg, Command};
use race_core::{
    transport::TransportT,
    types::{CreateRegistrationParams, GameBundle, GetRegistrationParams},
};
use race_env::Config;
use race_transport::{TransportBuilder, TransportError};
use std::{fs::File, io::Read};

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

async fn create_transport(
    config: &Config,
    chain: &str,
) -> Result<Box<dyn TransportT>, TransportError> {
    TransportBuilder::default()
        .try_with_chain(chain)?
        .try_with_config(config)?
        .build()
        .await
}

async fn publish(config: Config, chain: &str, bundle: &str) {
    let transport = create_transport(&config, chain)
        .await
        .expect("Failed to create transport");
    let mut file = File::open(bundle).unwrap();
    let mut buf = Vec::with_capacity(0x4000);
    file.read_to_end(&mut buf).unwrap();
    let addr = "facade-program-addr".into();
    let bundle = GameBundle { addr, data: buf };
    let resp = transport.publish_game(bundle).await.expect("RPC error");
    println!("Address: {:?}", &resp);
}

async fn bundle_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain)
        .await
        .expect("Failed to create transport");
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
    let transport = create_transport(&config, chain)
        .await
        .expect("Failed to create transport");
    match transport.get_game_account(addr).await {
        Some(game_account) => {
            println!("Game account: {:?}", game_account.addr);
            println!("Game bundle: {:?}", game_account.bundle_addr);
            println!("Access version: {:?}", game_account.access_version);
            println!("Settle version: {:?}", game_account.settle_version);
            println!("Data size: {:?}", game_account.data.len());
            println!("Players:");
            for p in game_account.players.iter() {
                println!("Player[{:?}] position: {:?}", p.addr, p.position);
            }
            println!("Deposits:");
            for d in game_account.deposits.iter() {
                println!("Deposit: from[{:?}], amount: {:?}", d.addr, d.amount);
            }
        }
        None => {
            println!("Game bundle not found");
        }
    }
}

async fn reg_info(config: Config, chain: &str, addr: &str) {
    let transport = create_transport(&config, chain)
        .await
        .expect("Failed to create transport");
    match transport
        .get_registration(GetRegistrationParams {
            addr: addr.to_owned(),
        })
        .await
    {
        Some(reg) => {
            println!("Registration account: {:?}", reg.addr);
            println!("Size(Registered): {:?}({:?})", reg.size, reg.games.len());
            println!("Owner: {:?}", reg.owner.unwrap_or("None".into()));
            println!("Games:");
            for g in reg.games.iter() {
                println!(
                    "Game account: {:?}, Game bundle: {:?}",
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
    let transport = create_transport(&config, chain)
        .await
        .expect("Failed to create transport");
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
