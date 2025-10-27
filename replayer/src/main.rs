mod utils;
mod error;
mod context;
mod server;
mod ui;
mod records_file_loader;
mod session_manager;
mod session;

use borsh::BorshDeserialize;
use clap::{arg, Command};
use crate::context::ReplayerContext;
use crate::error::ReplayerError;
use crate::server::run_server;
use crate::ui::render_controller_ui;
use crate::utils::base64_decode;
use race_env::Config;
use race_event_record::{Record, RecordsHeader};
use std::fs::File;
use std::io::{self, BufRead};
use std::sync::Arc;
use tracing::info;
use records_file_loader::load_event_records_from_file;

fn cli() -> Command {
    Command::new("replayer")
        .about("Replay the game events")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(Command::new("play")
            .about("Replay events from a file")
            .arg(arg!(<FILE> "The event records file")))
        .subcommand(Command::new("run")
            .about("Run replayer as a web server"))
}

async fn play(context: ReplayerContext, file: String) -> Result<(), ReplayerError> {
    let event_records = load_event_records_from_file(&context, file.into())?;

    let server_handle = run_server(context).await?;

    // render_controller_ui(context, event_records)?;

    server_handle.stop()?;

    Ok(())
}

async fn run(context: ReplayerContext) -> Result<(), ReplayerError> {
    let server_handle = run_server(context).await?;

    server_handle.stop()?;

    Ok(())
}

#[tokio::main]
pub async fn main() {
    let matches = cli().get_matches();

    let config = Config::from_path(&matches.get_one::<String>("config").unwrap().into()).await;
    let context = ReplayerContext::new(config);

    match matches.subcommand() {
        Some(("play", sub_matches)) => {
            let file = sub_matches.get_one::<String>("FILE").expect("required");
            play(context, file.into()).await.expect("An error occurred while playing events");
        }
        Some(("run", sub_matches)) => {
            run(context).await.expect("An error occurred while running as web server");
        }
        _ => {
            panic!("A valid sub command is required");
        }
    }
}
