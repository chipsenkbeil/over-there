pub mod connect;
pub mod route;
pub mod state;

use over_there_transport::TransceiverThread;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub struct Client {
    state: Arc<Mutex<state::ClientState>>,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<Vec<u8>, ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Client {
    pub fn heartbeat(&self) {}
}
