mod action;
pub mod file;
pub mod proc;
pub mod state;

use crate::{event::AddrEventManager, Communicator, Msg, Transport};
use log::error;
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, NetTransmission, OutboundWire, Signer, Verifier,
};
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
    /// Used to spawn jobs when communicating with clients
    handle: Handle,

    /// Address of bound server
    addr: SocketAddr,

    /// Represents the event manager used to send and receive data
    addr_event_manager: AddrEventManager,

    /// Represents the handle for processing events
    event_handle: task::JoinHandle<()>,
}

impl Server {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

async fn tcp_event_handler(mut rx: mpsc::Receiver<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>) {
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
    mut rx: mpsc::Receiver<(Msg, SocketAddr, mpsc::Sender<(Vec<u8>, SocketAddr)>)>,
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

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer + Send + 'static,
    V: Verifier + Send + 'static,
    E: Encrypter + Send + 'static,
    D: Decrypter + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(
        self,
        handle: Handle,
        transport: Transport,
        buffer: usize,
    ) -> io::Result<Server> {
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
                    listener.ok_or(io::Error::from(io::ErrorKind::AddrNotAvailable))?
                };
                let addr = listener.local_addr()?;

                let inbound_wire = InboundWire::new(
                    NetTransmission::TcpEthernet.into(),
                    self.packet_ttl,
                    self.verifier,
                    self.decrypter,
                );
                let outbound_wire = OutboundWire::new(
                    NetTransmission::TcpEthernet.into(),
                    self.signer,
                    self.encrypter,
                );

                let (tx, rx) = mpsc::channel(buffer);
                let event_handle = handle.spawn(tcp_event_handler(rx));
                let addr_event_manager = AddrEventManager::for_tcp_listener(
                    handle.clone(),
                    buffer,
                    listener,
                    inbound_wire,
                    outbound_wire,
                    tx,
                );

                Ok(Server {
                    handle,
                    addr,
                    addr_event_manager,
                    event_handle,
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
                    socket.ok_or(io::Error::from(io::ErrorKind::AddrNotAvailable))?
                };
                let addr = socket.local_addr()?;
                let transmission = NetTransmission::udp_from_addr(addr);

                let inbound_wire = InboundWire::new(
                    transmission.into(),
                    self.packet_ttl,
                    self.verifier,
                    self.decrypter,
                );
                let outbound_wire =
                    OutboundWire::new(transmission.into(), self.signer, self.encrypter);

                let (tx, rx) = mpsc::channel(buffer);
                let event_handle = handle.spawn(udp_event_handler(rx));
                let addr_event_manager = AddrEventManager::for_udp_socket(
                    handle.clone(),
                    buffer,
                    socket,
                    inbound_wire,
                    outbound_wire,
                    tx,
                );

                Ok(Server {
                    handle,
                    addr,
                    addr_event_manager,
                    event_handle,
                })
            }
        }
    }
}
