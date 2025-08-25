use async_trait::async_trait;
use tokio::{
    sync::mpsc::{self, error::SendError},
    task::JoinHandle,
};
use tracing::{info, warn};

use crate::{frame::EventFrame, utils::addr_shorthand};

use super::event_bus::CloseReason;

/// An interface for a component that can be attached to the event bus.
pub trait Attachable {
    fn id(&self) -> &str;

    /// Return the input channel of current component.
    /// Return `None` when the component does not accept input.
    fn input(&mut self) -> Option<mpsc::Sender<EventFrame>>;

    /// Return the output channel of this component.
    /// A component must return an output channel, even though it doesn't produce an output.
    /// A closed output channel means that this component has stopped.
    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>>;
}

/// Represent the input/output of the ports
pub struct PortsIO {
    input_tx: Option<mpsc::Sender<EventFrame>>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
}

impl PortsIO {
    #[allow(unused)]
    pub async fn send(&self, frame: EventFrame) -> Result<(), SendError<EventFrame>> {
        if let Some(ref input_tx) = self.input_tx {
            input_tx.send(frame).await
        } else {
            panic!("Input is not supported");
        }
    }

    #[allow(unused)]
    pub async fn recv(&mut self) -> Option<EventFrame> {
        if let Some(ref mut output_rx) = self.output_rx {
            output_rx.recv().await
        } else {
            panic!("Output is not supported");
        }
    }
}

pub struct PortsHandle {
    pub id: String,
    input_tx: Option<mpsc::Sender<EventFrame>>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    join_handle: JoinHandle<CloseReason>,
}

impl PortsHandle {
    fn from_io<S: Into<String>>(
        id: S,
        value: PortsIO,
        join_handle: JoinHandle<CloseReason>,
    ) -> Self {
        Self {
            id: id.into(),
            input_tx: value.input_tx,
            output_rx: value.output_rx,
            join_handle,
        }
    }
}

impl PortsHandle {
    pub async fn wait(self) -> CloseReason {
        match self.join_handle.await {
            Ok(close_reason) => close_reason,
            Err(e) => CloseReason::Fault(race_core::error::Error::InternalError(format!(
                "Error in waiting close reason: {:?}",
                e
            ))),
        }
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
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn input(&mut self) -> Option<mpsc::Sender<EventFrame>> {
        if self.input_tx.is_some() {
            self.input_tx.clone()
        } else {
            None
        }
    }
    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        if self.output_rx.is_some() {
            self.output_rx.take()
        } else {
            None
        }
    }
}

pub trait Ports: Send {
    fn create() -> (Self, PortsIO)
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
    fn create() -> (Self, PortsIO)
    where
        Self: Sized,
    {
        let (input_tx, input_rx) = mpsc::channel(100);
        (
            Self { rx: input_rx },
            PortsIO {
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
    #[allow(dead_code)]
    pub async fn try_send(&self, frame: EventFrame) -> Result<(), SendError<EventFrame>> {
        self.tx.send(frame).await
    }

    #[allow(dead_code)]
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
    fn create() -> (Self, PortsIO)
    where
        Self: Sized,
    {
        let (output_tx, output_rx) = mpsc::channel(10);
        (
            Self { tx: output_tx },
            PortsIO {
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

    pub fn clone_as_producer(&self) -> ProducerPorts {
        ProducerPorts {
            tx: self.tx.clone(),
        }
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
    fn create() -> (Self, PortsIO)
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
            PortsIO {
                input_tx: Some(input_tx),
                output_rx: Some(output_rx),
            },
        )
    }
}

#[allow(unused)]
pub struct ComponentEnv {
    pub addr: String,
    pub addr_shorthand: String,
    pub component_name: String,
    pub log_prefix: String,
}

impl ComponentEnv {
    pub fn new(addr: &str, component_name: &str) -> Self {
        let addr_short = addr_shorthand(addr);
        Self {
            addr: addr.into(),
            addr_shorthand: addr_short.clone(),
            log_prefix: format!("[{}|{}]", addr_short, component_name),
            component_name: component_name.into(),
        }
    }
}

#[async_trait]
pub trait Component<P, C>
where
    P: Ports + 'static,
    C: Send + 'static,
{
    fn name() -> &'static str;

    fn prepare(&self, addr: &str) -> (P, PortsIO, ComponentEnv) {
        let (ports, io) = P::create();
        let env = ComponentEnv::new(addr, Self::name());
        (ports, io, env)
    }

    fn start(&self, addr: &str, context: C) -> PortsHandle {
        info!("Starting component: {}", Self::name());
        let (ports, io, env) = self.prepare(addr);
        let join_handle = tokio::spawn(async move { Self::run(ports, context, env).await });
        PortsHandle::from_io(Self::name(), io, join_handle)
    }


    async fn run(ports: P, context: C, env: ComponentEnv) -> CloseReason;
}
