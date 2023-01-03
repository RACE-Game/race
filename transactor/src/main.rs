mod component;
mod handle;
mod context;
mod server;
mod reg;
mod frame;

use crate::server::run_server;
use clap::{arg, Command};
use context::ApplicationContext;
use race_env::Config;
use tokio::sync::Mutex;

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
    let context = Mutex::new(ApplicationContext::new(config).await);
    match matches.subcommand() {
        Some(("run", _)) => {
            run_server(context).await.expect("Unexpected error occured");
        }
        Some(("reg", _)) => {
        }
        _ => unreachable!()
    }
}
