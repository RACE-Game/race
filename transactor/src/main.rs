mod utils;
mod game_manager;
mod component;
mod context;
mod frame;
mod handle;
mod reg;
mod blacklist;
mod server;

use crate::server::run_server;
use clap::{arg, Command};
use context::ApplicationContext;
use race_env::Config;
use reg::{register_server, start_reg_task};
use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, Layer, EnvFilter};

fn cli() -> Command {
    Command::new("transactor")
        .about("Transactor server of Race Protocol.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(Command::new("run").about("Run server"))
        .subcommand(Command::new("reg").about("Register server account"))
}

fn setup_logger() {
    let logfile = tracing_appender::rolling::daily("logs", "transactor.log");
    let file_layer = fmt::layer()
        .with_writer(logfile)
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .with_filter(EnvFilter::from_default_env());
    let console_layer = fmt::layer()
        .compact()
        .with_writer(std::io::stdout)
        .with_level(true)
        .with_ansi(true)
        .without_time()
        .with_filter(EnvFilter::from_default_env());
    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to configure logger");
}

#[tokio::main]
pub async fn main() {

    setup_logger();

    let matches = cli().get_matches();
    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;
    match matches.subcommand() {
        Some(("run", _)) => {
            let context = ApplicationContext::try_new(config)
                .await
                .expect("Failed to initalize");
            start_reg_task(&context).await;
            run_server(context).await.expect("Unexpected error occured");
        }
        Some(("reg", _)) => {
            register_server(&config)
                .await
                .expect("Unexpected error occured");
        }
        _ => unreachable!(),
    }
}
