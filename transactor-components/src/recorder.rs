use async_trait::async_trait;
use borsh::{BorshSerialize, BorshDeserialize};
use race_core::context::Node;
use race_api::event::Event;
use race_transactor_frames::EventFrame;
use std::fs::{File, create_dir};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use super::ComponentEnv;
use super::common::ConsumerPorts;
use tokio::sync::Mutex;
use tracing::{info, error};

use crate::common::Component;
use crate::event_bus::CloseReason;
use crate::utils::base64_encode;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum Record {
    Checkpoint {
        state: Vec<u8>,
        nodes: Vec<Node>,
    },
    Event {
        event: Event,
        timestamp: u64,
    }
}

trait RecordWriter {
    fn write(&mut self, record: Record);
}

struct InMemoryRecordWriter {
    pub records: Vec<Record>,
}

impl InMemoryRecordWriter {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }
}

impl RecordWriter for InMemoryRecordWriter {

    fn write(&mut self, record: Record) {
        self.records.push(record);
    }
}

struct FileRecordWriter {
    pub writer: BufWriter<File>,
}

impl FileRecordWriter {
    pub fn try_new(record_file_name: PathBuf) -> std::io::Result<Self> {
        let file = File::create(record_file_name)?;
        let writer = BufWriter::new(file);
        Ok(Self {
            writer
        })
    }
}

impl FileRecordWriter {
    fn write_internal(&mut self, record: Record) -> std::io::Result<()> {
        let s = borsh::to_vec(&record)?;
        writeln!(&mut self.writer, "{}", base64_encode(&s))?;
        self.writer.flush()
    }
}

impl RecordWriter for FileRecordWriter {
    fn write(&mut self, record: Record) {
        if let Err(e) = self.write_internal(record) {
            error!("Failed to write record: {}", e);
        }
    }
}

pub struct Recorder {
    #[allow(unused)]
    addr: String,
    #[allow(unused)]
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
}

pub struct RecorderContext {
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
}

impl Recorder {
    pub fn init(
        addr: String,
        in_mem: bool,
    ) -> (Self, RecorderContext) {
        let writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>;

        if in_mem {
            writer = Arc::new(Mutex::new(InMemoryRecordWriter::new()))
        } else {
            let dir = Path::new("records");
            if !dir.exists() {
                create_dir(dir).expect("Failed to create records directory");
            }
            let file_path = format!("records/{}", addr);
            writer = Arc::new(Mutex::new(FileRecordWriter::try_new(file_path.into()).expect("Fail to create record writer")))
        }

        (
           Self { addr, writer: writer.clone() },
           RecorderContext {
               writer
           }
        )
    }
}

#[async_trait]
impl Component<ConsumerPorts, RecorderContext> for Recorder {
    fn name() -> &'static str {
        "Recorder"
    }

    async fn run(
        mut ports: ConsumerPorts,
        ctx: RecorderContext,
        env: ComponentEnv,
    ) -> CloseReason {

        let RecorderContext { writer } = ctx;

        while let Some(event_frame) = ports.recv().await {
            info!("{} Handle event frame: {}", env.log_prefix, event_frame);

            let mut writer = writer.lock().await;
            match event_frame {

                EventFrame::Broadcast {
                    event,
                    timestamp,
                    ..
                } => {
                    let record = Record::Event {
                        event, timestamp
                    };

                    writer.write(record);
                }

                EventFrame::Shutdown => {
                    break;
                }

                _ => ()
            }
        }

        CloseReason::Complete
    }
}
