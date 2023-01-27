use std::mem::swap;

use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;

/// An interface for a component that has a name.
pub trait Named {
    fn name<'a>(&self) -> &'a str;
}

/// An interface for a component that can be attached to the event bus.
pub trait Attachable {
    /// Return the input channel of current component.
    /// Returning `None` means that the component does not accept input.
    fn input(&self) -> Option<mpsc::Sender<EventFrame>>;

    /// Return the output channel of this component.
    /// A component must return an output channel, even though it doesn't produce an output.
    /// A closed output channel means that this component has stopped.
    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>>;
}

/// An interface for a component.
pub trait Component<C>: Named {
    /// Start the lifecycle of component.
    fn run(&mut self, ctx: C);

    /// Take the execution context.
    fn borrow_mut_ctx(&mut self) -> &mut Option<C>;

    /// Return a oneshot channel which will be closed when the component is terminated.
    fn closed(self) -> oneshot::Receiver<CloseReason>;

    /// Start this component.
    fn start(&mut self) {
        let mut ctx = None;
        swap(self.borrow_mut_ctx(), &mut ctx);
        if let Some(ctx) = ctx {
            info!("Start component {}", self.name());
            self.run(ctx);
        }
    }
}
