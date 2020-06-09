use super::{AddrEventManager, EventManager};
use crate::core::Msg;

use log::error;
use crate::wire::{
    Authenticator, Bicrypter, Decrypter, Encrypter, Signer,
    TcpStreamInboundWire, TcpStreamOutboundWire, Verifier, Wire,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Handle,
    sync::{mpsc, Mutex},
};

/// Implementation of EventManager for TCP stream
impl EventManager {
    pub fn for_tcp_stream<A, B>(
        handle: Handle,
        max_outbound_queue: usize,
        stream: TcpStream,
        remote_addr: SocketAddr,
        wire: Wire<A, B>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    ) -> EventManager
    where
        A: Authenticator + Send + Sync + 'static,
        B: Bicrypter + Send + Sync + 'static,
    {
        let (reader, writer) =
            wire.with_tcp_stream(stream, remote_addr).arc_split();

        let (tx, rx) = mpsc::channel::<Vec<u8>>(max_outbound_queue);

        let inbound_handle = handle.spawn(tcp_stream_outbound_loop(rx, writer));
        let outbound_handle = handle.spawn(tcp_stream_inbound_loop(
            tx.clone(),
            reader,
            on_inbound_tx,
        ));

        EventManager {
            inbound_handle,
            outbound_handle,
            tx,
        }
    }
}

/// Implementation of AddrEventManager for TCP listener (requires Clone
/// on Authenticator and Bicrypter)
impl AddrEventManager {
    pub fn for_tcp_listener<A, B>(
        handle: Handle,
        max_outbound_queue: usize,
        listener: TcpListener,
        wire: Wire<A, B>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    ) -> AddrEventManager
    where
        A: Authenticator + Send + Sync + Clone + 'static,
        B: Bicrypter + Send + Sync + Clone + 'static,
    {
        let connections: Arc<
            Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>,
        > = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) =
            mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);

        let outbound_handle = handle
            .spawn(tcp_listener_outbound_loop(rx, Arc::clone(&connections)));

        let inbound_handle = handle.spawn(tcp_listener_inbound_loop(
            handle.clone(),
            listener,
            wire,
            connections,
            on_inbound_tx,
            max_outbound_queue,
        ));

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            tx,
        }
    }
}

/// Loops continuously, reading outbound data and sending it out over the wire
/// of the appropriate connection
async fn tcp_listener_outbound_loop(
    mut rx: mpsc::Receiver<(Vec<u8>, SocketAddr)>,
    connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>>,
) {
    while let Some((msg, addr)) = rx.recv().await {
        if let Some(stream) = connections.lock().await.get_mut(&addr) {
            if stream.send(msg).await.is_err() {
                error!("Failed to send to {}", addr);
            }
        }
    }
}

/// Loops continuously accepting new connections and spawning EventManager
/// instances to process incoming and outgoing msgs over each individual
/// TcpStream formed by a connection
async fn tcp_listener_inbound_loop<A, B>(
    handle: Handle,
    mut listener: TcpListener,
    wire: Wire<A, B>,
    connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>>,
    on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    max_outbound_queue: usize,
) where
    A: Authenticator + Send + Sync + Clone + 'static,
    B: Bicrypter + Send + Sync + Clone + 'static,
{
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                handle.spawn(tcp_listener_spawn_stream(
                    stream,
                    addr,
                    handle.clone(),
                    wire.clone(),
                    Arc::clone(&connections),
                    on_inbound_tx.clone(),
                    max_outbound_queue,
                ));
            }
            Err(x) => {
                error!("Listening for connections encountered error: {}", x);
                break;
            }
        }
    }
}

/// Spawns a new EventManager for the given TcpStream to process inbound and
/// outbound msgs, waits for the EventManager to conclude (when the stream
/// is closed), and cleans up
async fn tcp_listener_spawn_stream<A, B>(
    stream: TcpStream,
    addr: SocketAddr,
    handle: Handle,
    wire: Wire<A, B>,
    connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>>,
    on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    max_outbound_queue: usize,
) where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    let event_manager = EventManager::for_tcp_stream(
        handle,
        max_outbound_queue,
        stream,
        addr,
        wire,
        on_inbound_tx,
    );

    connections
        .lock()
        .await
        .insert(addr, event_manager.tx.clone());

    // Wait for the stream's event manager to exit,
    // and remove the connection once it does
    if let Err(x) = event_manager.wait().await {
        error!("Event manager exited badly: {}", x);
    }

    connections.lock().await.remove(&addr);
}

/// Loops continuously, reading outbound data and sending it out over the wire
async fn tcp_stream_outbound_loop<S, E>(
    mut rx: mpsc::Receiver<Vec<u8>>,
    mut writer: TcpStreamOutboundWire<S, E>,
) where
    S: Signer,
    E: Encrypter,
{
    while let Some(msg) = rx.recv().await {
        if let Err(x) = writer.write(&msg).await {
            error!("Failed to send: {}", x);
        }
    }
}

/// Loops continuously, reading inbound data and passing it along to be
/// processed by event handlers
async fn tcp_stream_inbound_loop<V, D>(
    tx: mpsc::Sender<Vec<u8>>,
    mut reader: TcpStreamInboundWire<V, D>,
    on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
) where
    V: Verifier,
    D: Decrypter,
{
    loop {
        let tx_2 = tx.clone();
        let result = reader.read().await;
        if !super::process_inbound(result, tx_2, on_inbound_tx.clone()).await {
            break;
        }
    }
}
