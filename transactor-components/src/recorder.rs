use async_trait::async_trait;
use borsh::BorshSerialize;
use race_core::game_spec::GameSpec;
use race_core::entry_type::EntryType;
use race_core::chain::ChainType;
use race_transactor_frames::EventFrame;
use race_event_record::{RecordsHeader, Record};
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
    #[allow(unused)]
    game_id: usize,
}

impl Recorder {
    pub fn init(
        spec: GameSpec,
        init_data: Vec<u8>,
        entry_type: EntryType,
        chain: ChainType,
        in_mem: bool,
    ) -> (Self, RecorderContext) {
        let writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>;
        let game_id = spec.game_id;

        let header = RecordsHeader::new(spec.clone(), init_data, entry_type, chain.to_string());

        if in_mem {
            writer = Arc::new(Mutex::new(InMemoryRecordWriter::new(header)))
        } else {
            let dir = Path::new("records");
            if !dir.exists() {
                create_dir(dir).expect("Failed to create records directory");
            }
            let file_path = if spec.game_id == 0 {
                format!("records/{}", spec.game_addr)
            } else {
                format!("records/{}_{}", spec.game_addr, spec.game_id)
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

        let RecorderContext { writer, .. } = ctx;

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
                    checkpoint: _,
                } => {
                    // XXX fix recorder
                    //
                    // if let Some(state) = checkpoint.versioned_data(game_id).map(|vd| vd.handler_state) {
                    //     let nodes = checkpoint.shared_data().nodes.clone();

                    //     let record = Record::Checkpoint {
                    //         state, nodes, access_version, settle_version, balances,
                    //     };

                    //     writer.write(record);
                    // }
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
