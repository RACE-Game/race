use async_trait::async_trait;
use borsh::BorshSerialize;
use race_transactor_frames::EventFrame;
use race_event_record::{RecordsHeader, Record};
use race_core::chain::ChainType;
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

trait RecordWriter {
    fn write(&mut self, record: Record);
}

struct InMemoryRecordWriter {
    #[allow(unused)]
    pub header: RecordsHeader,
    pub records: Vec<Record>,
}

impl InMemoryRecordWriter {
    pub fn new(header: RecordsHeader) -> Self {
        Self {
            header,
            records: Vec::new()
        }
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
    pub fn try_new(record_file_name: PathBuf, header: RecordsHeader) -> std::io::Result<Self> {
        let file = File::create(record_file_name)?;
        let writer = BufWriter::new(file);
        let mut file_writer = Self { writer };
        if let Err(e) = file_writer.write_internal(&header) {
            error!("Failed to write header: {}", e);
            return Err(e);
        }
        Ok(file_writer)
    }
}

impl FileRecordWriter {
    fn write_internal<B: BorshSerialize>(&mut self, b: &B) -> std::io::Result<()> {
        let s = borsh::to_vec(b)?;
        writeln!(&mut self.writer, "{}", base64_encode(&s))?;
        self.writer.flush()
    }
}

impl RecordWriter for FileRecordWriter {
    fn write(&mut self, record: Record) {
        if let Err(e) = self.write_internal(&record) {
            error!("Failed to write record: {}", e);
        }
    }
}

pub struct Recorder {
    #[allow(unused)]
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
}

pub struct RecorderContext {
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
    game_id: usize,
}

impl Recorder {
    pub fn init(
        game_addr: String,
        game_id: usize,
        bundle_addr: String,
        chain: ChainType,
        in_mem: bool,
    ) -> (Self, RecorderContext) {
        let writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>;

        let header = RecordsHeader::new(game_addr.clone(), game_id, bundle_addr, chain.to_string());

        if in_mem {
            writer = Arc::new(Mutex::new(InMemoryRecordWriter::new(header)))
        } else {
            let dir = Path::new("records");
            if !dir.exists() {
                create_dir(dir).expect("Failed to create records directory");
            }
            let file_path = if game_id == 0 {
                format!("records/{}", game_addr)
            } else {
                format!("records/{}_{}", game_addr, game_id)
            };
            writer = Arc::new(Mutex::new(FileRecordWriter::try_new(file_path.into(), header).expect("Fail to create record writer")))
        }

        (
           Self { writer: writer.clone() },
           RecorderContext {
               writer,
               game_id,
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

        let RecorderContext { game_id, writer } = ctx;

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

                EventFrame::Checkpoint {
                    checkpoint,
                    nodes,
                    ..
                } => {
                    if let Some(state) = checkpoint.get_data(game_id) {
                        let record = Record::Checkpoint {
                            state, nodes
                        };

                        writer.write(record);
                    }
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
