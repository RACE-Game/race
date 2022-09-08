use clap::{arg, Command};
use race_core::{types::GameBundle, transport::TransportT};
use race_facade::FacadeTransport;
use std::{fs::File, io::Read};

fn create_transport(chain: &str) -> Box<dyn TransportT> {
    match chain {
        "facade" => Box::new(FacadeTransport::default()),
        _ => panic!("Unsupported chain"),
    }
}

fn cli() -> Command {
    Command::new("cli")
        .about("Command line tools for Race Protocol")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("publish")
                .about("Publish a game bundle")
                .arg(arg!(<CHAIN> "The chain to interact"))
                .arg(arg!(<BUNDLE> "The path to the WASM bundle"))
                .arg_required_else_help(true),
        )
}

async fn publish(bundle: &str, chain: &str) {
    let transport = create_transport(chain);
    let mut file = File::open(bundle).unwrap();
    let mut buf = Vec::with_capacity(0x4000);
    file.read_to_end(&mut buf).unwrap();
    let addr = "facade-program-addr".into();
    let bundle = GameBundle { addr, data: buf };
    let resp = transport.publish_game(bundle).await.expect("RPC error");
    println!("Address: {:?}", &resp);
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let chain = sub_matches.get_one::<String>("CHAIN").expect("required");
            let bundle = sub_matches.get_one::<String>("BUNDLE").expect("required");
            publish(bundle, chain).await;
        }
        _ => unreachable!(),
    }
}
