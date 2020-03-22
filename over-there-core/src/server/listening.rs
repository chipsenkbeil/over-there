use crate::event::AddrEventManager;
use std::net::SocketAddr;
use tokio::task::{JoinError, JoinHandle};

/// Represents a server after listening has begun
pub struct ListeningServer {
    /// Address of bound server
    pub(super) addr: SocketAddr,

    /// Represents the event manager used to send and receive data
    pub(super) addr_event_manager: AddrEventManager,

    /// Represents the handle for processing events
    pub(super) event_handle: JoinHandle<()>,
}

impl ListeningServer {
    pub fn addr_event_manager(&self) -> &AddrEventManager {
        &self.addr_event_manager
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn wait(self) -> Result<(), JoinError> {
        tokio::try_join!(self.addr_event_manager.wait(), self.event_handle)
            .map(|_| ())
    }
}
