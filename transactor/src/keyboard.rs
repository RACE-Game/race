///! Keyboard handling

use tokio::{signal, task::JoinHandle};

use crate::context::ApplicationContext;
use race_transactor_frames::SignalFrame;

#[allow(unused)]
pub fn setup_keyboard_handler(context: &mut ApplicationContext) -> JoinHandle<()> {

    let signal_tx = context.get_signal_sender();

    let ctrl_c_handler = tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        signal_tx.send(SignalFrame::Shutdown).await.expect("Failed to send shutdown signal");
    });

    ctrl_c_handler
}
