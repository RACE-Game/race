//! Read records file, return EventRecords

use crate::error::ReplayerError;
use crate::context::ReplayerContext;
use crate::utils::base64_decode;
use borsh::BorshDeserialize;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::fs::File;
use race_transport::builder::TransportBuilder;
use race_event_record::{RecordsHeader, EventRecords, Record};

pub fn load_event_records_from_file(context: &ReplayerContext, file: PathBuf) -> Result<EventRecords, ReplayerError> {
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

    Ok(EventRecords::new(header, records))
}
