mod component;
mod context;
mod server;

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

pub async fn run(config: Config) {
    run_server(config).await.expect("Unexpected error occured");
}

#[tokio::main]
pub async fn main() {
    let matches = cli().get_matches();
    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;
    match matches.subcommand() {
        Some(("run", _)) => {
            run(config).await;
        }
        Some(("reg", _)) => {
        }
        _ => unreachable!()
    }
}
