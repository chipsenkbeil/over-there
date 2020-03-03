mod action;
pub mod fs;
pub mod proc;
pub mod state;

use crate::{event::AddrEventManager, Communicator, Msg, Transport};
use log::error;
use over_there_wire::{Authenticator, Bicrypter, NetTransmission, Wire};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
    runtime::Handle,
    sync::mpsc,
    task,
};

/// Represents a server after listening has begun
pub struct Server {
    /// Address of bound server
    addr: SocketAddr,

    /// Represents the event manager used to send and receive data
    addr_event_manager: AddrEventManager,

    /// Represents the handle for processing events
    _event_handle: task::JoinHandle<()>,
}

impl Server {
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

async fn tcp_event_handler(
    mut rx: mpsc::Receiver<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
) {
    let state = Arc::new(state::ServerState::default());
    while let Some((msg, addr, tx)) = rx.recv().await {
        if let Err(x) = action::Executor::<Vec<u8>>::new(tx, addr)
            .execute(Arc::clone(&state), msg)
            .await
        {
            error!("Failed to execute action: {}", x);
        }
    }
}

async fn udp_event_handler(
    mut rx: mpsc::Receiver<(
        Msg,
        SocketAddr,
        mpsc::Sender<(Vec<u8>, SocketAddr)>,
    )>,
) {
    let state = Arc::new(state::ServerState::default());
    while let Some((msg, addr, tx)) = rx.recv().await {
        if let Err(x) = action::Executor::<(Vec<u8>, SocketAddr)>::new(tx, addr)
            .execute(Arc::clone(&state), msg)
            .await
        {
            error!("Failed to execute action: {}", x);
        }
    }
}

impl<A, B> Communicator<A, B>
where
    A: Authenticator + Clone + Send + 'static,
    B: Bicrypter + Clone + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(
        self,
        transport: Transport,
        buffer: usize,
    ) -> io::Result<Server> {
        let handle = Handle::current();

        match transport {
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

                let (tx, rx) = mpsc::channel(buffer);
                let _event_handle = handle.spawn(tcp_event_handler(rx));
                let addr_event_manager = AddrEventManager::for_tcp_listener(
                    handle.clone(),
                    buffer,
                    listener,
                    wire,
                    tx,
                );

                Ok(Server {
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

                let (tx, rx) = mpsc::channel(buffer);
                let _event_handle = handle.spawn(udp_event_handler(rx));
                let addr_event_manager = AddrEventManager::for_udp_socket(
                    handle.clone(),
                    buffer,
                    socket,
                    wire,
                    tx,
                );

                Ok(Server {
                    addr,
                    addr_event_manager,
                    _event_handle,
                })
            }
        }
    }
}
