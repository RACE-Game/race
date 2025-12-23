mod utils;
mod error;
mod context;
mod server;
mod ui;

use borsh::BorshDeserialize;
use clap::{arg, Command};
use crate::context::ReplayerContext;
use crate::error::ReplayerError;
use crate::server::run_server;
use crate::ui::render_controller_ui;
use crate::utils::base64_decode;
use race_env::Config;
use race_event_record::{Record, RecordsHeader};
use race_transport::builder::TransportBuilder;
use std::fs::File;
use std::io::{self, BufRead};
use std::sync::Arc;

fn cli() -> Command {
    Command::new("replayer")
        .about("Replay the game events")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(-c <config> "The path to config file").default_value("config.toml"))
        .subcommand(Command::new("play")
            .about("Replay events from a file")
            .arg(arg!(<FILE> "The event records file")))
}

async fn play(context: ReplayerContext, file: String) -> Result<(), ReplayerError> {
    let port = context.config.replayer.as_ref().expect("Missing replayer config").port;

    let context = Arc::new(context);

    let mut lines = io::BufReader::new(File::open(file)?).lines();
    let Some(Ok(header_line)) = lines.next() else {
        return Err(ReplayerError::MissingHeader);
    };

    let header = RecordsHeader::try_from_slice(&base64_decode(&header_line)?)?;

    let transport = TransportBuilder::default()
        .with_chain(header.chain.as_str().into())
        .try_with_config(&context.config)?
        .build();

    let mut records = vec![];
    for ln in lines{
        let ln = ln?;
        let v = base64_decode(&ln)?;
        let r = Record::try_from_slice(&v)?;
        records.push(r);
    }

    let server_handle = run_server(context.clone()).await?;

    render_controller_ui(context, header, records)?;

    server_handle.stop()?;

    return Ok(())
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
        _ => {
            panic!("A valid sub command is required");
        }
    }
}
