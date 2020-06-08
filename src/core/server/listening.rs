use super::state::ServerState;
use crate::core::event::AddrEventManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::{JoinError, JoinHandle};

/// Represents a server after listening has begun
pub struct ListeningServer {
    /// Address of bound server
    pub(super) addr: SocketAddr,

    /// Represents the event manager used to send and receive data
    pub(super) addr_event_manager: AddrEventManager,

    /// Represents the state of the active server
    pub(super) state: Arc<ServerState>,

    /// Represents the handle for processing events
    pub(super) event_handle: JoinHandle<()>,
}

impl ListeningServer {
    /// Represents the manager of inbound and outbound msgs
    pub fn addr_event_manager(&self) -> &AddrEventManager {
        &self.addr_event_manager
    }

    /// Represents the bound address of the server
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Flags the server's internal state as no longer running, closing down
    /// all running tasks
    pub fn shutdown(&self) {
        self.state.shutdown()
    }

    /// Waits for the server to complete
    pub async fn wait(self) -> Result<(), JoinError> {
        tokio::try_join!(self.addr_event_manager.wait(), self.event_handle)
            .map(|_| ())
    }
}
