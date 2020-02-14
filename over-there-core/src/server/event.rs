use super::{action, state::ServerState};
use crate::Msg;

use log::{error, trace, warn};
use over_there_wire::{Decrypter, InboundWire, InboundWireError, Verifier};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::{
    io,
    net::{tcp, TcpListener, UdpSocket},
    runtime::Handle,
    sync::mpsc,
    task,
};

pub struct Loops {
    pub send_handle: task::JoinHandle<()>,
    pub event_handle: task::JoinHandle<()>,
    pub tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
}

pub fn spawn_tcp_loops<V, D>(
    handle: Handle,
    buffer: usize,
    listener: TcpListener,
    inbound_wire: InboundWire<V, D>,
    state: ServerState,
) -> Loops
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    let mut connections: HashMap<SocketAddr, tcp::WriteHalf> = HashMap::new();

    let (tx, rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(buffer);
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
                    let (r_h, w_h) = stream.split();
                    connections.insert(addr, w_h);
                    let _ = handle.spawn(async {
                        loop {
                            let result = inbound_wire
                                .async_recv(|buf| {
                                    use futures::future::FutureExt;
                                    use io::AsyncReadExt;
                                    r_h.read(buf).map(|res| res.map(|size| (size, addr)))
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

    Loops {
        send_handle,
        event_handle,
        tx,
    }
}

pub fn spawn_udp_loops<V, D>(
    handle: Handle,
    buffer: usize,
    socket: UdpSocket,
    inbound_wire: InboundWire<V, D>,
    state: ServerState,
) -> Loops
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(buffer);
    let (r_h, s_h) = socket.split();
    let send_handle = handle.spawn(async {
        while let Some((msg, addr)) = rx.recv().await {
            if let Err(x) = s_h.send_to(&msg, &addr).await {
                error!("Failed to send: {}", x);
            }
        }
    });
    let event_handle = handle.spawn(async {
        loop {
            let result = inbound_wire.async_recv(|buf| r_h.recv_from(buf)).await;
            if !process_recv(&mut state, result, tx).await {
                break;
            }
        }
    });

    Loops {
        send_handle,
        event_handle,
        tx,
    }
}

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_recv(
    state: &mut ServerState,
    result: Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>,
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some((data, addr))) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);
                match action::execute(state, &msg, tx, addr).await {
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
