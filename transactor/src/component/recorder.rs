use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::component::common::Component;
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;

use tracing::info;

use super::common::ConsumerPorts;
use super::ComponentEnv;

trait RecordWriter {
    fn write(&mut self, event_frame: EventFrame);

    fn dump_records(&self) -> Vec<EventFrame>;
}

struct InMemoryRecordWriter {
    pub records: Vec<EventFrame>,
}

impl InMemoryRecordWriter {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }
}

impl RecordWriter for InMemoryRecordWriter {
    fn write(&mut self, event_frame: EventFrame) {
        self.records.push(event_frame);
    }

    fn dump_records(&self) -> Vec<EventFrame> {
        self.records.clone()
    }
}

pub struct Recorder {
    #[allow(unused)]
    addr: String,
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
}

pub struct RecorderContext {
    writer: Arc<Mutex<dyn RecordWriter + Send + Sync>>,
}

impl Recorder {
    pub fn init(
        addr: String,
        _in_mem: bool,
    ) -> (Self, RecorderContext) {

        let writer = InMemoryRecordWriter::new();
        let writer = Arc::new(Mutex::new(writer));

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
                EventFrame::Shutdown => {
                    writer.write(event_frame);
                    break;
                }

                _ => {
                    writer.write(event_frame);
                }
            }
        }

        CloseReason::Complete
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recorder() -> anyhow::Result<()> {
        let (recorder, recorder_ctx) = Recorder::init("test_addr".into(), true);
        // let mut recorder_handle = recorder.start("test_addr", recorder_ctx);

        let (ports, io, env) = recorder.prepare("test_addr");

        io.send(EventFrame::Empty).await?;
        io.send(EventFrame::Shutdown).await?;

        let _close_reason = Recorder::run(ports, recorder_ctx, env).await;

        let writer = recorder.writer.lock().await;
        assert!(matches!(writer.dump_records()[0], EventFrame::Empty));
        assert!(matches!(writer.dump_records()[1], EventFrame::Shutdown));

        Ok(())
    }

}
