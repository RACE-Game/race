mod utils;
mod game_manager;
mod component;
mod context;
mod frame;
mod handle;
mod reg;
mod blacklist;
mod server;
mod keyboard;

use tracing::error;
use crate::server::run_server;
use clap::{arg, Command};
use context::ApplicationContext;
use keyboard::setup_keyboard_handler;
use race_env::Config;
use reg::{register_server, start_reg_task};
use tokio::try_join;
use tracing::info;
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

fn setup_logger(config: &Config) {
    let logdir = config.transactor.as_ref().and_then(|c| c.log_dir.as_ref()).map(String::as_str).unwrap_or("logs");
    let logfile = tracing_appender::rolling::daily(&logdir, "transactor.log");
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

    let matches = cli().get_matches();
    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;

    setup_logger(&config);

    match matches.subcommand() {
        Some(("run", _)) => {
            info!("Starting transactor.");
            let (mut context, signal_loop) = ApplicationContext::try_new_and_start_signal_loop(config)
                .await
                .expect("Failed to initalize");
            let keyboard_listener = setup_keyboard_handler(&mut context);
            let reg_task = start_reg_task(&context).await;
            let server_handle = run_server(context).await.expect("Unexpected error occured");
            if let Err(e) = try_join!(signal_loop, keyboard_listener, reg_task, server_handle) {
                error!("Error: {:?}", e);
            }
            info!("Transactor stopped");
        }
        Some(("reg", _)) => {
            info!("Register server profile for current account.");
            register_server(&config)
                .await
                .expect("Unexpected error occured");
        }
        _ => unreachable!(),
    }
}
