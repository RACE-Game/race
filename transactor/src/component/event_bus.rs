use std::sync::Arc;

use race_api::error::Error;
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{error, warn};

use crate::component::common::Attachable;
use crate::frame::EventFrame;
use crate::utils::addr_shorthand;

/// An event bus that passes the events between different components.
pub struct EventBus {
    #[allow(unused)]
    addr: String,
    tx: mpsc::Sender<EventFrame>,
    attached_txs: Arc<Mutex<Vec<(String, mpsc::Sender<EventFrame>)>>>,
    close_rx: watch::Receiver<bool>,
}

impl EventBus {
    pub fn new(addr: String) -> Self {
        let (close_tx, close_rx) = watch::channel(false);
        let (tx, mut rx) = mpsc::channel::<EventFrame>(32);
        let txs: Arc<Mutex<Vec<(String, mpsc::Sender<EventFrame>)>>> = Arc::new(Mutex::new(vec![]));
        let attached_txs = txs.clone();
        let addr_1 = addr_shorthand(&addr);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let txs = attached_txs.lock().await;
                for (id, t) in txs.iter() {
                    if t.send(msg.clone()).await.is_err() {
                        warn!(
                            "[{}] Failed to send message: {} to component: {}",
                            addr_1,
                            msg,
                            id
                        );
                    }
                }
                if matches!(msg, EventFrame::Shutdown) {
                    close_tx.send(true).unwrap();
                    break;
                }
            }
        });
        Self {
            addr,
            tx,
            attached_txs: txs,
            close_rx,
        }
    }

    pub async fn attach<T>(&self, attachable: &mut T)
    where
        T: Attachable,
    {
        let mut close_rx = self.close_rx.clone();
        if let Some(mut rx) = attachable.output() {
            let tx = self.tx.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = close_rx.changed() => {
                            break;
                        }
                        msg = rx.recv() => {
                            if let Some(msg) = msg {
                                match tx.send(msg).await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        error!("Failed to send event: {:?}", e);
                                        warn!("Shutdown event bus");
                                        return;
                                    }
                                }

                            } else {
                                break;
                            }
                        }
                    }
                }
            });
        }

        if let Some(tx) = attachable.input() {
            let mut txs = self.attached_txs.lock().await;
            txs.push((attachable.id().to_string(), tx.clone()));
        }
    }

    pub async fn send(&self, event: EventFrame) {
        // info!("Event bus receive event frame: {:?}", event);
        if let Err(e) = self.tx.send(event).await {
            error!("An error occurred when sending event, {}", e.to_string());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

/// A data represent the reason of closing.
#[derive(Debug, Clone)]
pub enum CloseReason {
    Complete,
    Fault(Error),
}

#[cfg(test)]
mod tests {

    use crate::component::{common::{Component, ConsumerPorts, ProducerPorts}, ComponentEnv};

    use super::*;
    use async_trait::async_trait;
    use tokio::time::{sleep, Duration};

    #[derive(Default)]
    struct TestProducerCtx {}

    #[derive(Default)]
    struct TestProducer {}

    #[async_trait]
    impl Component<ProducerPorts, TestProducerCtx> for TestProducer {
        fn name() -> &'static str {
            "Test Producer"
        }

        async fn run(ports: ProducerPorts, _ctx: TestProducerCtx, _env: ComponentEnv) -> CloseReason {
            loop {
                println!("Producer started");
                let event = EventFrame::Sync {
                    new_players: vec![],
                    new_servers: vec![],
                    new_deposits: vec![],
                    transactor_addr: "".into(),
                    access_version: 1,
                };
                if ports.try_send(event.clone()).await.is_ok() {
                    sleep(Duration::from_millis(1)).await;
                } else {
                    break;
                }
            }
            CloseReason::Complete
        }
    }

    struct TestConsumerCtx {
        n: Arc<Mutex<u8>>,
    }

    struct TestConsumer {
        n: Arc<Mutex<u8>>,
    }

    impl TestConsumer {
        pub fn init() -> (Self, TestConsumerCtx) {
            let n = Arc::new(Mutex::new(0));
            (Self { n: n.clone() }, TestConsumerCtx { n })
        }
        pub fn get_n(&self) -> Arc<Mutex<u8>> {
            self.n.clone()
        }
    }

    #[async_trait]
    impl Component<ConsumerPorts, TestConsumerCtx> for TestConsumer {
        fn name() -> &'static str {
            "Test Consumer"
        }

        async fn run(mut ports: ConsumerPorts, ctx: TestConsumerCtx, _env: ComponentEnv) -> CloseReason {
            println!("Consumer started");
            loop {
                match ports.recv().await {
                    Some(event) => {
                        println!("Consumer receive event: {:?}", event);
                        let mut n = ctx.n.lock().await;
                        *n += 1;
                        println!("n = {:?}", n);
                        if *n == 2 {
                            break;
                        }
                    }
                    None => {
                        println!("Consumer input closed!");
                    }
                }
            }
            println!("Consumer quit");
            return CloseReason::Complete;
        }
    }

    // #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    #[tokio::test]
    async fn test_component_produce() {
        let p = TestProducer::default();
        let p_ctx = TestProducerCtx::default();
        let (c, c_ctx) = TestConsumer::init();
        let eb = EventBus::default();

        let mut p_handle = p.start("producer", p_ctx,);
        let mut c_handle = c.start("consumer", c_ctx);

        eb.attach(&mut p_handle).await;
        eb.attach(&mut c_handle).await;

        let n = c.get_n();

        c_handle.wait().await;

        let n = n.lock().await;
        assert_eq!(*n, 2);
    }
}
