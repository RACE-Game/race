use std::sync::Arc;

use race_core::error::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::component::traits::Attachable;
use crate::frame::EventFrame;

/// An event bus that passes the events between different components.
pub struct EventBus {
    tx: mpsc::Sender<EventFrame>,
    attached_txs: Arc<Mutex<Vec<mpsc::Sender<EventFrame>>>>,
}

impl EventBus {
    pub async fn attach<T: Attachable>(&self, attachable: &mut T) {
        if let Some(mut rx) = attachable.output() {
            let tx = self.tx.clone();
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    match tx.send(msg).await {
                        Ok(_) => (),
                        Err(e) => {
                            error!("Failed to send event: {:?}", e);
                        }
                    }
                }
            });
        }
        if let Some(tx) = attachable.input() {
            let mut txs = self.attached_txs.lock().await;
            txs.push(tx.clone());
        }
    }

    pub async fn send(&self, event: EventFrame) {
        info!("Event bus receive event frame: {:?}", event);
        if let Err(e) = self.tx.send(event).await {
            error!("An error occurred when sending event, {}", e.to_string());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        let (tx, mut rx) = mpsc::channel::<EventFrame>(32);
        let txs: Arc<Mutex<Vec<mpsc::Sender<EventFrame>>>> = Arc::new(Mutex::new(vec![]));
        let attached_txs = txs.clone();

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                info!("Dispatching message: {:?}", msg);
                let txs = attached_txs.lock().await;
                for t in txs.iter() {
                    t.send(msg.clone()).await.unwrap();
                }
            }
        });
        Self {
            tx,
            attached_txs: txs,
        }
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

    use super::*;
    use crate::component::traits::{Attachable, Component, Named};
    use tokio::{
        sync::{oneshot, mpsc},
        time::{sleep, Duration},
    };

    struct TestProducerCtx {
        output_tx: mpsc::Sender<EventFrame>,
        closed_tx: oneshot::Sender<CloseReason>,
    }

    struct TestProducer {
        output_rx: Option<mpsc::Receiver<EventFrame>>,
        closed_rx: oneshot::Receiver<CloseReason>,
        ctx: Option<TestProducerCtx>,
    }

    impl Named for TestProducer {
        fn name<'a>(&self) -> &'a str {
            "TestProducer"
        }
    }

    impl Attachable for TestProducer {
        fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
            None
        }

        fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
            let mut ret = None;
            std::mem::swap(&mut ret, &mut self.output_rx);
            ret
        }
    }

    impl Component<TestProducerCtx> for TestProducer {
        fn closed(self) -> oneshot::Receiver<CloseReason> {
            self.closed_rx
        }

        fn run(&mut self, ctx: TestProducerCtx) {
            tokio::spawn(async move {
                loop {
                    println!("Producer started");
                    let event = EventFrame::PlayerJoined {
                        new_players: vec![],
                    };
                    match ctx.output_tx.send(event.clone()).await {
                        Ok(_) => sleep(Duration::from_secs(5)).await,
                        Err(_) => {
                            break;
                        }
                    }
                }
                ctx.closed_tx.send(CloseReason::Complete).unwrap();
            });
        }

        fn borrow_mut_ctx(&mut self) -> &mut Option<TestProducerCtx> {
            &mut self.ctx
        }
    }

    impl TestProducer {
        fn new() -> Self {
            let (output_tx, output_rx) = mpsc::channel(3);
            let (closed_tx, closed_rx) = oneshot::channel();
            let ctx = TestProducerCtx {
                output_tx,
                closed_tx,
            };
            Self {
                output_rx: Some(output_rx),
                closed_rx,
                ctx: Some(ctx),
            }
        }
    }

    struct TestConsumerCtx {
        input_rx: mpsc::Receiver<EventFrame>,
        output_tx: mpsc::Sender<EventFrame>,
        closed_tx: oneshot::Sender<CloseReason>,
        n: Arc<Mutex<u8>>,
    }

    struct TestConsumer {
        input_tx: mpsc::Sender<EventFrame>,
        output_rx: Option<mpsc::Receiver<EventFrame>>,
        closed_rx: oneshot::Receiver<CloseReason>,
        ctx: Option<TestConsumerCtx>,
        n: Arc<Mutex<u8>>,
    }

    impl Component<TestConsumerCtx> for TestConsumer {
        fn run(&mut self, mut ctx: TestConsumerCtx) {
            tokio::spawn(async move {
                println!("Consumer started");
                loop {
                    match ctx.input_rx.recv().await {
                        Some(event) => {
                            println!("Consumer receive event: {:?}", event);
                            let mut n = ctx.n.lock().await;
                            *n += 1;
                            println!("n = {:?}", n);
                            if *n == 2 {
                                break;
                            } else {
                                ctx.output_tx.send(EventFrame::Empty).await.unwrap();
                            }
                        }
                        None => {
                            println!("Consumer input closed!");
                        }
                    }
                }
                println!("Consumer quit");
                ctx.closed_tx.send(CloseReason::Complete).unwrap();
            });
        }

        fn closed(self) -> oneshot::Receiver<CloseReason> {
            self.closed_rx
        }

        fn borrow_mut_ctx(&mut self) -> &mut Option<TestConsumerCtx> {
            &mut self.ctx
        }
    }

    impl Named for TestConsumer {
        fn name<'a>(&self) -> &'a str {
            "TestConsumer"
        }
    }

    impl Attachable for TestConsumer {
        fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
            Some(self.input_tx.clone())
        }

        fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
            let mut ret = None;
            std::mem::swap(&mut ret, &mut self.output_rx);
            ret
        }
    }

    impl TestConsumer {
        fn new() -> Self {
            let (input_tx, input_rx) = mpsc::channel(1);
            let (output_tx, output_rx) = mpsc::channel(3);
            let (closed_tx, closed_rx) = oneshot::channel();
            let n = Arc::new(Mutex::new(0));

            let ctx = TestConsumerCtx {
                input_rx,
                output_tx,
                closed_tx,
                n: n.clone(),
            };

            Self {
                input_tx,
                output_rx: Some(output_rx),
                closed_rx,
                ctx: Some(ctx),
                n,
            }
        }

        pub fn get_n(&self) -> Arc<Mutex<u8>> {
            self.n.clone()
        }
    }

    // #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    #[tokio::test]
    async fn test_component_produce() {
        let mut p = TestProducer::new();
        let mut c = TestConsumer::new();
        let eb = EventBus::default();

        eb.attach(&mut c).await;
        eb.attach(&mut p).await;

        c.start();
        p.start();

        let n = c.get_n();

        c.closed().await.unwrap();

        let n = n.lock().await;
        assert_eq!(*n, 2);
    }
}
