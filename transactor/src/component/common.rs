use async_trait::async_trait;
use tokio::{
    sync::mpsc::{self, error::SendError},
    task::JoinHandle,
};
use tracing::{info, warn};

use crate::frame::EventFrame;

use super::event_bus::CloseReason;

/// An interface for a component that can be attached to the event bus.
pub trait Attachable {
    /// Return the input channel of current component.
    /// Return `None` when the component does not accept input.
    fn input(&mut self) -> Option<mpsc::Sender<EventFrame>>;

    /// Return the output channel of this component.
    /// A component must return an output channel, even though it doesn't produce an output.
    /// A closed output channel means that this component has stopped.
    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>>;
}

/// The group of channels to be attached to an event bus.
pub struct PortsHandleInner {
    input_tx: Option<mpsc::Sender<EventFrame>>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
}

pub struct PortsHandle {
    input_tx: Option<mpsc::Sender<EventFrame>>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    join_handle: JoinHandle<CloseReason>,
}

impl PortsHandle {
    fn from_inner(value: PortsHandleInner, join_handle: JoinHandle<CloseReason>) -> Self {
        Self {
            input_tx: value.input_tx,
            output_rx: value.output_rx,
            join_handle,
        }
    }
}

impl PortsHandle {
    pub async fn wait(self) -> CloseReason {
        self.join_handle.await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn send_unchecked(&self, frame: EventFrame) {
        if let Some(ref input_tx) = self.input_tx {
            input_tx.send(frame).await.expect("Failed to send");
        } else {
            panic!("Sender is not available");
        }
    }

    #[allow(dead_code)]
    pub async fn recv_unchecked(&mut self) -> Option<EventFrame> {
        if let Some(ref mut output_rx) = self.output_rx {
            output_rx.recv().await
        } else {
            panic!("Receiver is not available");
        }
    }
}

impl Attachable for PortsHandle {
    fn input(&mut self) -> Option<mpsc::Sender<EventFrame>> {
        if self.input_tx.is_some() {
            self.input_tx.clone()
        } else {
            None
        }
    }
    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        if self.output_rx.is_some() {
            std::mem::replace(&mut self.output_rx, None)
        } else {
            None
        }
    }
}

pub trait Ports: Send {
    fn create() -> (Self, PortsHandleInner)
    where
        Self: Sized;
}

pub struct ConsumerPorts {
    rx: mpsc::Receiver<EventFrame>,
}

impl ConsumerPorts {
    pub async fn recv(&mut self) -> Option<EventFrame> {
        self.rx.recv().await
    }
}

impl Ports for ConsumerPorts {
    fn create() -> (Self, PortsHandleInner)
    where
        Self: Sized,
    {
        let (input_tx, input_rx) = mpsc::channel(100);
        (
            Self { rx: input_rx },
            PortsHandleInner {
                input_tx: Some(input_tx),
                output_rx: None,
            },
        )
    }
}

pub struct ProducerPorts {
    tx: mpsc::Sender<EventFrame>,
}

impl ProducerPorts {
    pub async fn try_send(&self, frame: EventFrame) -> Result<(), SendError<EventFrame>> {
        self.tx.send(frame).await
    }

    pub fn is_tx_closed(&self) -> bool {
        self.tx.is_closed()
    }

    pub async fn send(&self, frame: EventFrame) {
        match self.tx.send(frame).await {
            Ok(_) => (),
            Err(e) => {
                warn!("Send error: {:?}", e)
            }
        }
    }
}

impl Ports for ProducerPorts {
    fn create() -> (Self, PortsHandleInner)
    where
        Self: Sized,
    {
        let (output_tx, output_rx) = mpsc::channel(10);
        (
            Self { tx: output_tx },
            PortsHandleInner {
                input_tx: None,
                output_rx: Some(output_rx),
            },
        )
    }
}

pub struct PipelinePorts {
    rx: mpsc::Receiver<EventFrame>,
    tx: mpsc::Sender<EventFrame>,
}

impl PipelinePorts {
    pub async fn recv(&mut self) -> Option<EventFrame> {
        self.rx.recv().await
    }

    #[allow(unused)]
    pub async fn try_send(&self, frame: EventFrame) -> Result<(), SendError<EventFrame>> {
        self.tx.send(frame).await
    }

    pub async fn send(&self, frame: EventFrame) {
        match self.tx.send(frame).await {
            Ok(_) => (),
            Err(e) => {
                warn!("Send error: {:?}", e)
            }
        }
    }
}

impl Ports for PipelinePorts {
    fn create() -> (Self, PortsHandleInner)
    where
        Self: Sized,
    {
        let (input_tx, input_rx) = mpsc::channel(10);
        let (output_tx, output_rx) = mpsc::channel(10);
        (
            Self {
                rx: input_rx,
                tx: output_tx,
            },
            PortsHandleInner {
                input_tx: Some(input_tx),
                output_rx: Some(output_rx),
            },
        )
    }
}

#[async_trait]
pub trait Component<P, C>
where
    P: Ports + 'static,
    C: Send + 'static,
{
    fn name(&self) -> &str;

    fn start(&self, context: C) -> PortsHandle {
        info!("Starting component: {}", self.name());
        let (ports, ports_handle_inner) = P::create();
        let join_handle = tokio::spawn(async move {
            Self::run(ports, context).await
        });
        PortsHandle::from_inner(ports_handle_inner, join_handle)
    }
    async fn run(ports: P, context: C) -> CloseReason;
}
