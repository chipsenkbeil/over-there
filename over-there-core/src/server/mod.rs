mod action;
mod event;
pub mod file;
pub mod proc;
pub mod state;

use crate::{Communicator, Transport};
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, NetTransmission, OutboundWire, Signer, Verifier,
};
use std::net::SocketAddr;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
    runtime::Runtime,
    sync::mpsc,
    task,
};

/// Represents a server after listening has begun
pub struct Server {
    state: state::ServerState,

    /// Used to spawn jobs when communicating with clients
    runtime: Runtime,

    /// Primary event handle processing incoming msgs
    event_handle: task::JoinHandle<()>,

    /// Primary send handle processing outgoing msgs
    send_handle: task::JoinHandle<()>,

    /// Means to send new outbound msgs
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
}

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer,
    V: Verifier + Send + 'static,
    E: Encrypter,
    D: Decrypter + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(self, transport: Transport, buffer: usize) -> io::Result<Server> {
        let runtime = Runtime::new()?;
        let handle = runtime.handle();
        let mut state = state::ServerState::default();

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

                let event::Loops {
                    event_handle,
                    send_handle,
                    tx,
                } = event::spawn_tcp_loops(handle.clone(), buffer, listener, inbound_wire, state);

                Ok(Server {
                    state,
                    runtime,
                    event_handle,
                    send_handle,
                    tx,
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

                let event::Loops {
                    event_handle,
                    send_handle,
                    tx,
                } = event::spawn_udp_loops(handle.clone(), buffer, socket, inbound_wire, state);

                Ok(Server {
                    state,
                    runtime,
                    event_handle,
                    send_handle,
                    tx,
                })
            }
        }
    }
}
