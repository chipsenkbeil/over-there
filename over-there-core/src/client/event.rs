use super::state::ClientState;
use crate::Msg;

use log::{error, trace, warn};
use over_there_auth::Verifier;
use over_there_crypto::Decrypter;
use over_there_wire::{InboundWire, InboundWireError};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::{
    io,
    net::{TcpStream, UdpSocket},
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
    handle: &Handle,
    buffer: usize,
    inbound_wire: InboundWire<V, D>,
    stream: TcpStream,
    remote_addr: SocketAddr,
    state: &mut ClientState,
) -> Loops
where
    V: Verifier + Send,
    D: Decrypter + Send,
{
    let mut connections: HashMap<SocketAddr, TcpStream> = HashMap::new();

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
            let result = inbound_wire
                .async_recv(|buf| {
                    use futures::future::FutureExt;
                    use io::AsyncReadExt;
                    stream
                        .read(buf)
                        .map(|res| res.map(|size| (size, remote_addr)))
                })
                .await;
            if !process_recv(state, result).await {
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

pub fn spawn_udp_loops<V, D>(
    handle: &Handle,
    buffer: usize,
    inbound_wire: InboundWire<V, D>,
    socket: UdpSocket,
    state: &mut ClientState,
) -> Loops
where
    V: Verifier + Send,
    D: Decrypter + Send,
{
    let (tx, rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(buffer);
    let send_handle = handle.spawn(async {
        while let Some((msg, addr)) = rx.recv().await {
            if let Err(x) = socket.send_to(&msg, addr).await {
                error!("Failed to send: {}", x);
                break;
            }
        }
    });

    let event_handle = handle.spawn(async {
        loop {
            let result = inbound_wire.async_recv(|buf| socket.recv_from(buf)).await;
            if !process_recv(&mut state, result).await {
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
    state: &mut ClientState,
    result: Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>,
) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some((data, addr))) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);
                // TODO: Invoke callback
                true
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
