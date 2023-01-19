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
use reg::{start_reg_task, register_server};
use tokio::sync::Mutex;

fn cli() -> Command {
    Command::new("transactor")
        .about("Transactor server of Race Protocol.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(Command::new("run").about("Run server"))
        .subcommand(Command::new("reg").about("Register server account"))
}

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();

    let matches = cli().get_matches();
    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;
    match matches.subcommand() {
        Some(("run", _)) => {
            let context = Mutex::new(ApplicationContext::try_new(config).await.expect("Failed to initalize"));
            start_reg_task(&context).await;
            run_server(context).await.expect("Unexpected error occured");
        }
        Some(("reg", _)) => {
            register_server(&config).await.expect("Unexpected error occured");
        }
        _ => unreachable!()
    }
}
