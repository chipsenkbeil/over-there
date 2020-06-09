mod action;
mod custom;
pub mod fs;
mod listening;
pub mod proc;
pub mod state;

pub use listening::ListeningServer;

use crate::core::{event::AddrEventManager, Msg, Transport};
use derive_builder::Builder;
use log::error;
use crate::transport::{Authenticator, Bicrypter, NetTransmission, Wire};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
    runtime::Handle,
    sync::mpsc,
    time,
};

/// Represents a server configuration prior to listening
#[derive(Builder, Clone)]
pub struct Server<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    /// TTL to collect all packets for a msg
    #[builder(default = "crate::transport::constants::DEFAULT_TTL")]
    packet_ttl: Duration,

    /// Used to sign & verify msgs
    authenticator: A,

    /// Used to encrypt & decrypt msgs
    bicrypter: B,

    /// Transportation mechanism & address to listen on
    transport: Transport,

    /// Internal buffer for cross-thread messaging
    #[builder(default = "1000")]
    buffer: usize,

    /// Interval at which cleanup of dangling resources is performed
    #[builder(default = "Duration::from_secs(60)")]
    cleanup_interval: Duration,

    /// TTL for an untouched, open file before it is closed during cleanup
    #[builder(default = "state::constants::DEFAULT_FILE_TTL")]
    file_ttl: Duration,

    /// TTL for an untouched, running process before it is killed during cleanup
    #[builder(default = "state::constants::DEFAULT_PROC_TTL")]
    proc_ttl: Duration,

    /// TTL for an untouched, dead process before it is removed during cleanup
    #[builder(default = "state::constants::DEFAULT_DEAD_PROC_TTL")]
    dead_proc_ttl: Duration,
}

impl<A, B> Server<A, B>
where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    ///
    /// Will fail if using TCP transport as requires Clone; should instead
    /// use `cloneable_listen` if using TCP
    pub async fn listen(self) -> io::Result<ListeningServer> {
        let handle = Handle::current();
        let state = Arc::new(state::ServerState::new(
            self.file_ttl,
            self.proc_ttl,
            self.dead_proc_ttl,
        ));

        handle.spawn(cleanup_loop(Arc::clone(&state), self.cleanup_interval));

        match self.transport.clone() {
            Transport::Tcp(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Authenticator or Bicrypter is not clonable",
            )),
            Transport::Udp(addrs) => {
                build_and_listen_udp_server(self, state, &addrs).await
            }
        }
    }
}

impl<A, B> Server<A, B>
where
    A: Authenticator + Send + Sync + Clone + 'static,
    B: Bicrypter + Send + Sync + Clone + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium,
    /// using cloneable methods for Authenticator and Bicrypter operations
    pub async fn cloneable_listen(self) -> io::Result<ListeningServer> {
        let handle = Handle::current();
        let state = Arc::new(state::ServerState::new(
            self.file_ttl,
            self.proc_ttl,
            self.dead_proc_ttl,
        ));

        handle.spawn(cleanup_loop(Arc::clone(&state), self.cleanup_interval));

        match self.transport.clone() {
            Transport::Tcp(addrs) => {
                build_and_listen_tcp_server(self, state, &addrs).await
            }
            Transport::Udp(addrs) => {
                build_and_listen_udp_server(self, state, &addrs).await
            }
        }
    }
}

async fn build_and_listen_tcp_server<A, B>(
    server: Server<A, B>,
    state: Arc<state::ServerState>,
    addrs: &[SocketAddr],
) -> io::Result<ListeningServer>
where
    A: Authenticator + Send + Sync + Clone + 'static,
    B: Bicrypter + Send + Sync + Clone + 'static,
{
    let handle = Handle::current();

    // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
    //       so we have to loop through manually
    // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
    let listener = {
        let mut listener = None;
        for addr in addrs.iter() {
            let result = TcpListener::bind(addr).await;
            if result.is_ok() {
                listener = result.ok();
                break;
            }
        }
        listener
            .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?
    };
    let addr = listener.local_addr()?;

    let wire = Wire::new(
        NetTransmission::TcpEthernet.into(),
        server.packet_ttl,
        server.authenticator,
        server.bicrypter,
    );

    let (tx, rx) = mpsc::channel(server.buffer);
    let event_handle = handle.spawn(tcp_event_loop(Arc::clone(&state), rx));
    let addr_event_manager = AddrEventManager::for_tcp_listener(
        handle.clone(),
        server.buffer,
        listener,
        wire,
        tx,
    );

    Ok(ListeningServer {
        addr,
        addr_event_manager,
        state,
        event_handle,
    })
}

async fn build_and_listen_udp_server<A, B>(
    server: Server<A, B>,
    state: Arc<state::ServerState>,
    addrs: &[SocketAddr],
) -> io::Result<ListeningServer>
where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    let handle = Handle::current();

    // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
    //       so we have to loop through manually
    // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
    let socket = {
        let mut socket = None;
        for addr in addrs.iter() {
            let result = UdpSocket::bind(addr).await;
            if result.is_ok() {
                socket = result.ok();
                break;
            }
        }
        socket
            .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?
    };
    let addr = socket.local_addr()?;
    let transmission = NetTransmission::udp_from_addr(addr);

    let wire = Wire::new(
        transmission.into(),
        server.packet_ttl,
        server.authenticator,
        server.bicrypter,
    );

    let (tx, rx) = mpsc::channel(server.buffer);
    let event_handle = handle.spawn(udp_event_loop(Arc::clone(&state), rx));
    let addr_event_manager = AddrEventManager::for_udp_socket(
        handle.clone(),
        server.buffer,
        socket,
        wire,
        tx,
    );

    Ok(ListeningServer {
        addr,
        addr_event_manager,
        state,
        event_handle,
    })
}

async fn tcp_event_loop(
    state: Arc<state::ServerState>,
    mut rx: mpsc::Receiver<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
) {
    while let Some((msg, addr, tx)) = rx.recv().await {
        if let Err(x) = action::Executor::<Vec<u8>>::new(
            tx,
            addr,
            action::Executor::<Vec<u8>>::DEFAULT_MAX_DEPTH,
        )
        .execute(Arc::clone(&state), msg)
        .await
        {
            error!("Failed to execute action: {}", x);
        }
    }
}

async fn udp_event_loop(
    state: Arc<state::ServerState>,
    mut rx: mpsc::Receiver<(
        Msg,
        SocketAddr,
        mpsc::Sender<(Vec<u8>, SocketAddr)>,
    )>,
) {
    while let Some((msg, addr, tx)) = rx.recv().await {
        if let Err(x) = action::Executor::<(Vec<u8>, SocketAddr)>::new(
            tx,
            addr,
            action::Executor::<(Vec<u8>, SocketAddr)>::DEFAULT_MAX_DEPTH,
        )
        .execute(Arc::clone(&state), msg)
        .await
        {
            error!("Failed to execute action: {}", x);
        }
    }
}

async fn cleanup_loop(state: Arc<state::ServerState>, period: Duration) {
    while state.is_running() {
        state.evict_files().await;
        state.evict_procs().await;
        time::delay_for(period).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cleanup_loop_should_evict_unused_files_every_period() {
        let state = Arc::new(state::ServerState::default());

        // Touch some file ids so we can verify that the loop will remove
        // some of them, but don't bother opening them as full evict is
        // tested elsewhere
        state
            .touch_file_id_with_ttl(0, Duration::from_micros(5))
            .await;
        state
            .touch_file_id_with_ttl(1, Duration::from_secs(60))
            .await;

        // Run loop with a very short period so we can ensure we check quickly
        Handle::current()
            .spawn(cleanup_loop(Arc::clone(&state), Duration::from_millis(1)));

        // Ensure that we've waited long enough for some files to be evicted
        time::delay_for(Duration::from_millis(10)).await;

        // Verify expected files were evicted
        let file_ids = state.file_ids.lock().await;
        assert!(!file_ids.contains(&From::from(0)), "File not evicted");
        assert!(
            file_ids.contains(&From::from(1)),
            "File unexpectedly evicted"
        );
    }

    #[tokio::test]
    async fn cleanup_loop_should_evict_unused_processes_every_period() {
        let state = Arc::new(state::ServerState::default());

        // Touch some proc ids so we can verify that the loop will remove
        // some of them, but don't bother spawning them as full evict is
        // tested elsewhere
        state
            .touch_proc_id_with_ttl(0, Duration::from_micros(5))
            .await;
        state
            .touch_proc_id_with_ttl(1, Duration::from_secs(60))
            .await;

        // Run loop with a very short period so we ensure we check quickly
        Handle::current()
            .spawn(cleanup_loop(Arc::clone(&state), Duration::from_millis(1)));

        // Ensure that we've waited long enough for some procs to be evicted
        time::delay_for(Duration::from_millis(10)).await;

        // Verify expected files were evicted
        let proc_ids = state.proc_ids.lock().await;
        assert!(!proc_ids.contains(&From::from(0)), "Proc not evicted");
        assert!(
            proc_ids.contains(&From::from(1)),
            "Proc unexpectedly evicted"
        );
    }

    #[tokio::test]
    async fn cleanup_loop_should_only_run_if_state_marked_running() {
        let state = Arc::new(state::ServerState::default());

        // Shutdown before we even start to ensure that cleanup never happens
        state.shutdown();

        let state = Arc::new(state::ServerState::default());

        state.touch_file_id_with_ttl(0, Duration::new(0, 0)).await;
        state.touch_proc_id_with_ttl(0, Duration::new(0, 0)).await;

        Handle::current()
            .spawn(cleanup_loop(Arc::clone(&state), Duration::from_millis(1)));

        assert!(
            state.file_ids.lock().await.contains(&From::from(0)),
            "File unexpectedly evicted"
        );
        assert!(
            state.proc_ids.lock().await.contains(&From::from(0)),
            "Proc unexpectedly evicted"
        );
    }
}
