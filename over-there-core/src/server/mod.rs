mod action;
pub mod file;
pub mod proc;
pub mod state;

use crate::{event::AddrEventManager, Communicator, Transport};
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, NetTransmission, OutboundWire, Signer, Verifier,
};
use std::sync::Arc;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
    runtime::Runtime,
};

/// Represents a server after listening has begun
pub struct Server {
    /// Used to spawn jobs when communicating with clients
    runtime: Runtime,

    /// Represents the event manager used to send and receive data
    addr_event_manager: AddrEventManager,
}

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer + Send + 'static,
    V: Verifier + Send + 'static,
    E: Encrypter + Send + 'static,
    D: Decrypter + Send + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(self, transport: Transport, buffer: usize) -> io::Result<Server> {
        let runtime = Runtime::new()?;
        let handle = runtime.handle();
        let state = Arc::new(state::ServerState::default());

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

                let addr_event_manager = AddrEventManager::for_tcp_listener(
                    handle.clone(),
                    buffer,
                    listener,
                    inbound_wire,
                    outbound_wire,
                    move |(msg, _, mut tx)| {
                        use futures::future::TryFutureExt;
                        action::execute(Arc::clone(&state), msg, move |data: Vec<u8>| {
                            let data = data.to_vec();
                            async {
                                tx.send(data)
                                    .map_err(|_| {
                                        io::Error::new(
                                            io::ErrorKind::BrokenPipe,
                                            "Outbound communication closed",
                                        )
                                    })
                                    .await
                            }
                        })
                        .map_err(|x| x.into())
                    },
                );

                Ok(Server {
                    runtime,
                    addr_event_manager,
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

                let addr_event_manager = AddrEventManager::for_udp_socket(
                    handle.clone(),
                    buffer,
                    socket,
                    inbound_wire,
                    outbound_wire,
                    move |(msg, addr, mut tx)| {
                        use futures::future::TryFutureExt;
                        action::execute(Arc::clone(&state), msg, move |data: Vec<u8>| {
                            let data = data.to_vec();
                            async {
                                tx.send((data, addr))
                                    .map_err(|_| {
                                        io::Error::new(
                                            io::ErrorKind::BrokenPipe,
                                            "Outbound communication closed",
                                        )
                                    })
                                    .await
                            }
                        })
                        .map_err(|x| x.into())
                    },
                );

                Ok(Server {
                    runtime,
                    addr_event_manager,
                })
            }
        }
    }
}
