mod action;
pub mod fs;
pub mod proc;
pub mod state;

use crate::{event::AddrEventManager, Msg, Transport};
use derive_builder::Builder;
use log::error;
use over_there_wire::{Authenticator, Bicrypter, NetTransmission, Wire};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
    runtime::Handle,
    sync::mpsc,
    task, time,
};

/// Represents a server after listening has begun
pub struct ListeningServer {
    /// Address of bound server
    addr: SocketAddr,

    /// Represents the event manager used to send and receive data
    addr_event_manager: AddrEventManager,

    /// Represents the handle for processing events
    _event_handle: task::JoinHandle<()>,
}

impl ListeningServer {
    pub fn addr_event_manager(&self) -> &AddrEventManager {
        &self.addr_event_manager
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn wait(self) -> Result<(), task::JoinError> {
        tokio::try_join!(self.addr_event_manager.wait(), self._event_handle)
            .map(|_| ())
    }
}

async fn tcp_event_loop(
    state: Arc<state::ServerState>,
    mut rx: mpsc::Receiver<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
) {
    while let Some((msg, addr, tx)) = rx.recv().await {
        if let Err(x) = action::Executor::<Vec<u8>>::new(tx, addr)
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
        if let Err(x) = action::Executor::<(Vec<u8>, SocketAddr)>::new(tx, addr)
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

/// Represents a server configuration prior to listening
#[derive(Builder, Clone)]
pub struct Server<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    /// TTL to collect all packets for a msg
    #[builder(default = "over_there_wire::constants::DEFAULT_TTL")]
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
}

impl<A, B> Server<A, B>
where
    A: Authenticator + Clone + Send + 'static,
    B: Bicrypter + Clone + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(self) -> io::Result<ListeningServer> {
        let handle = Handle::current();
        let state = Arc::new(state::ServerState::default());

        // TODO: Provide config option for cleanup period (how often check occurs)
        handle.spawn(cleanup_loop(Arc::clone(&state), Duration::from_secs(60)));

        match self.transport {
            Transport::Tcp(addrs) => {
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
                    listener.ok_or_else(|| {
                        io::Error::from(io::ErrorKind::AddrNotAvailable)
                    })?
                };
                let addr = listener.local_addr()?;

                let wire = Wire::new(
                    NetTransmission::TcpEthernet.into(),
                    self.packet_ttl,
                    self.authenticator,
                    self.bicrypter,
                );

                let (tx, rx) = mpsc::channel(self.buffer);
                let _event_handle =
                    handle.spawn(tcp_event_loop(Arc::clone(&state), rx));
                let addr_event_manager = AddrEventManager::for_tcp_listener(
                    handle.clone(),
                    self.buffer,
                    listener,
                    wire,
                    tx,
                );

                Ok(ListeningServer {
                    addr,
                    addr_event_manager,
                    _event_handle,
                })
            }
            Transport::Udp(addrs) => {
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
                    socket.ok_or_else(|| {
                        io::Error::from(io::ErrorKind::AddrNotAvailable)
                    })?
                };
                let addr = socket.local_addr()?;
                let transmission = NetTransmission::udp_from_addr(addr);

                let wire = Wire::new(
                    transmission.into(),
                    self.packet_ttl,
                    self.authenticator,
                    self.bicrypter,
                );

                let (tx, rx) = mpsc::channel(self.buffer);
                let _event_handle =
                    handle.spawn(udp_event_loop(Arc::clone(&state), rx));
                let addr_event_manager = AddrEventManager::for_udp_socket(
                    handle.clone(),
                    self.buffer,
                    socket,
                    wire,
                    tx,
                );

                Ok(ListeningServer {
                    addr,
                    addr_event_manager,
                    _event_handle,
                })
            }
        }
    }
}
