mod component;
mod handle;
mod context;
mod server;
mod reg;

use crate::server::run_server;
use clap::{arg, Command};
use race_env::Config;

fn cli() -> Command {
    Command::new("transactor")
        .about("Transactor server of Race Protocol.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(Command::new("run"))
        .subcommand(Command::new("reg"))
}

#[tokio::main]
pub async fn main() {
    let matches = cli().get_matches();
    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;
    match matches.subcommand() {
        Some(("run", _)) => {
            run_server(config).await.expect("Unexpected error occured");
        }
        Some(("reg", _)) => {
        }
        _ => unreachable!()
    }
}
