mod action;
pub mod file;
pub mod proc;
pub mod state;

use crate::{Communicator, Msg, Transport};
use log::{error, trace, warn};
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, InboundWireError, NetTransmission, OutboundWire,
    OutboundWireError, Signer, Verifier,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::{
    io,
    net::{TcpListener, TcpStream, UdpSocket},
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
    tx: mpsc::UnboundedSender<(Vec<u8>, SocketAddr)>,
}

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn listen(self, transport: Transport) -> io::Result<Server> {
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
                let mut connections: HashMap<SocketAddr, TcpStream> = HashMap::new();

                let (tx, rx) = mpsc::unbounded_channel::<(Vec<u8>, SocketAddr)>();
                let send_handle = handle.spawn(async {
                    while let Some((msg, addr)) = rx.recv().await {
                        if let Some(stream) = connections.get_mut(&addr) {
                            use tokio::io::AsyncWriteExt;
                            if let Err(x) = stream.write_all(&msg).await {
                                error!("Failed to send: {}", x);
                                connections.remove(&addr);
                            }
                        }
                    }
                });
                let event_handle = handle.spawn(async {
                    loop {
                        match listener.accept().await {
                            Ok((stream, addr)) => {
                                connections.insert(addr, stream);
                                let _ = handle.spawn(async {
                                    loop {
                                        let result = inbound_wire
                                            .async_recv(|buf| {
                                                use futures::future::FutureExt;
                                                use io::AsyncReadExt;
                                                stream
                                                    .read(buf)
                                                    .map(|res| res.map(|size| (size, addr)))
                                            })
                                            .await;
                                        if !process_recv(&mut state, result, tx).await {
                                            break;
                                        }
                                    }

                                    connections.remove(&addr);
                                });
                            }
                            Err(x) => {
                                error!("Listening for connections encountered error: {}", x);
                                break;
                            }
                        }
                    }
                });

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

                let (tx, rx) = mpsc::unbounded_channel::<(Vec<u8>, SocketAddr)>();
                let send_handle = handle.spawn(async {
                    while let Some((msg, addr)) = rx.recv().await {
                        if let Err(x) = socket.send_to(&msg, addr).await {
                            error!("Failed to send: {}", x);
                        }
                    }
                });
                let event_handle = handle.spawn(async {
                    loop {
                        let result = inbound_wire.async_recv(|buf| socket.recv_from(buf)).await;
                        if !process_recv(&mut state, result, tx).await {
                            break;
                        }
                    }
                });

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

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_recv(
    state: &mut state::ServerState,
    result: Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>,
    tx: mpsc::UnboundedSender<(Vec<u8>, SocketAddr)>,
) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some((data, addr))) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);
                match action::execute(state, &msg, move |data| {
                    tx.send((data.to_vec(), addr)).map_err(|_| {
                        OutboundWireError::IO(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "Outbound communication closed",
                        ))
                    })
                })
                .await
                {
                    Ok(_) => true,
                    Err(action::ActionError::Unknown) => {
                        warn!("Unknown msg: {:?}", msg);
                        true
                    }
                    Err(x) => {
                        error!("Encountered error processing msg: {}", x);
                        true
                    }
                }
            } else {
                warn!("Discarding data of size {} as not valid msg", data.len());
                true
            }
        }
        Err(x) => match x {
            InboundWireError::IO(x) => {
                error!("Fatal IO on socket: {}", x);
                false
            }
            InboundWireError::InputProcessor(x) => {
                error!("Process error on socket: {}", x);
                true
            }
        },
    }
}
