mod component;
mod config;
mod context;
mod server;

use std::path::PathBuf;

use crate::server::run_server;
use clap::{arg, Command};
use config::load_config;

fn cli() -> Command {
    Command::new("transactor")
        .about("Transactor server of Race Protocol.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("run").arg(arg!([config] "The path to config file")))
        .subcommand(Command::new("reg").arg(arg!([config] "The path to config file")))
}

pub async fn run(path: &PathBuf) {
    let config = load_config(path).await;
    run_server(config).await.expect("Unexpected error occured");
}

#[tokio::main]
pub async fn main() {
    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", subcommand_matches)) => {
            let path = subcommand_matches.get_one::<PathBuf>("config");
            run(&path.unwrap_or(&"config.toml".into())).await;
        }
        Some(("reg", subcommand_matches)) => {
            let path = subcommand_matches.get_one::<PathBuf>("config");
        }
        _ => unreachable!()
    }
}
