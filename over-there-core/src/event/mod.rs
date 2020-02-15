use crate::Msg;

use log::{error, trace, warn};
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, InboundWireError, OutboundWire, Signer, Verifier,
};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use tokio::{
    io,
    net::{self, TcpListener, TcpStream, UdpSocket},
    runtime::Handle,
    sync::mpsc,
    task,
};

pub struct EventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    outbound_tx: mpsc::Sender<Vec<u8>>,
}

impl EventManager {
    pub fn outbound_tx(&self) -> &mpsc::Sender<Vec<u8>> {
        &self.outbound_tx
    }

    pub async fn send(&self, data: Vec<u8>) -> Result<(), Vec<u8>> {
        self.outbound_tx.send(data).await.map_err(|x| x.0)
    }

    pub async fn join(&self) {
        tokio::join!(self.inbound_handle, self.outbound_handle);
    }
}

pub struct AddrEventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    outbound_tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
}

impl AddrEventManager {
    pub fn outbound_tx(&self) -> &mpsc::Sender<(Vec<u8>, SocketAddr)> {
        &self.outbound_tx
    }

    pub async fn send_to(
        &self,
        data: Vec<u8>,
        addr: SocketAddr,
    ) -> Result<(), (Vec<u8>, SocketAddr)> {
        self.outbound_tx.send((data, addr)).await.map_err(|x| x.0)
    }

    pub async fn join(&self) {
        tokio::join!(self.inbound_handle, self.outbound_handle);
    }
}

struct InboundTcpStream<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    inbound_wire: InboundWire<V, D>,
    stream: io::ReadHalf<TcpStream>,
    remote_addr: SocketAddr,
}

impl<V, D> InboundTcpStream<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    pub async fn read(&mut self) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError> {
        use tokio::io::AsyncReadExt;

        let mut buf = vec![0; self.inbound_wire.transmission_size()].into_boxed_slice();
        let size = self
            .stream
            .read(&mut buf)
            .await
            .map_err(InboundWireError::IO)?;
        let data = self.inbound_wire.process(&buf)?;

        Ok((data, self.remote_addr))
    }
}

struct InboundUdpSocket<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    inbound_wire: InboundWire<V, D>,
    socket: net::udp::RecvHalf,
}

impl<V, D> InboundUdpSocket<V, D>
where
    V: Verifier + Send + 'static,
    D: Decrypter + Send + 'static,
{
    pub async fn read(&mut self) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError> {
        let mut buf = vec![0; self.inbound_wire.transmission_size()].into_boxed_slice();
        let (size, addr) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(InboundWireError::IO)?;
        let data = self.inbound_wire.process(&buf)?;

        Ok((data, addr))
    }
}

impl EventManager {
    pub fn for_tcp_stream<S, V, E, D, F, R>(
        handle: Handle,
        max_outbound_queue: usize,
        mut stream: TcpStream,
        remote_addr: SocketAddr,
        mut inbound_wire: InboundWire<V, D>,
        mut outbound_wire: OutboundWire<S, E>,
        mut on_inbound: F,
    ) -> EventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
        F: FnMut((Msg, SocketAddr, mpsc::Sender<Vec<u8>>)) -> R + Send + 'static,
        R: Future<Output = Result<(), Box<dyn std::error::Error>>> + Send,
    {
        // NOTE: Using io::split instead of TcpStream.split as the TcpStream
        //       uses a mutable reference (instead of taking ownership) and
        //       the returned halves are restricted to the stream's lifetime,
        //       meaning you can only run them on a single thread together
        let (mut stream_reader, mut stream_writer) = io::split(stream);

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            while let Some(msg) = rx.recv().await {
                use tokio::io::AsyncWriteExt;
                if let Err(x) = outbound_wire
                    .async_send(&msg, move |buf| {
                        let buf = buf.to_vec();
                        async { stream_writer.write(&buf).await }
                    })
                    .await
                {
                    error!("Failed to send: {}", x);
                }
            }
        });

        let handle_2 = handle.clone();
        let tx_2 = tx.clone();
        let inbound_handle = handle.spawn(async move {
            let reader = InboundTcpStream {
                inbound_wire,
                stream: stream_reader,
                remote_addr,
            };

            loop {
                let result = reader.read().await;
                if !process_inbound(handle_2, result, tx_2, on_inbound).await {
                    break;
                }
            }
        });

        EventManager {
            outbound_handle,
            inbound_handle,
            outbound_tx: tx,
        }
    }
}

impl AddrEventManager {
    pub fn for_udp_socket<S, V, E, D, F, R>(
        handle: Handle,
        max_outbound_queue: usize,
        socket: UdpSocket,
        mut inbound_wire: InboundWire<V, D>,
        mut outbound_wire: OutboundWire<S, E>,
        mut on_inbound: F,
    ) -> AddrEventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
        F: FnMut((Msg, SocketAddr, mpsc::Sender<(Vec<u8>, SocketAddr)>)) -> R + Send + 'static,
        R: Future<Output = Result<(), Box<dyn std::error::Error>>> + Send,
    {
        // NOTE: While this consumes the socket in v0.2.11 of Tokio, this appears
        //       to be a mistake and will be changed to &mut self in v0.3 as
        //       described here:
        //
        //       https://github.com/tokio-rs/tokio/pull/1630#issuecomment-559921381
        //
        let (mut socket_reader, mut socket_writer) = socket.split();

        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                if let Err(x) = outbound_wire
                    .async_send(&msg, |buf| {
                        let buf = buf.to_vec();
                        async { socket_writer.send_to(&buf, &addr).await }
                    })
                    .await
                {
                    error!("Failed to send: {}", x);
                    break;
                }
            }
        });

        let handle_2 = handle.clone();
        let tx_2 = tx.clone();
        let inbound_handle = handle.spawn(async move {
            let reader = InboundUdpSocket {
                inbound_wire,
                socket: socket_reader,
            };

            loop {
                let result = reader.read().await;
                if !process_inbound(handle_2, result, tx_2, on_inbound).await {
                    break;
                }
            }
        });

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            outbound_tx: tx,
        }
    }

    pub fn for_tcp_listener<S, V, E, D, F, R>(
        handle: Handle,
        max_outbound_queue: usize,
        listener: TcpListener,
        mut inbound_wire: InboundWire<V, D>,
        mut outbound_wire: OutboundWire<S, E>,
        mut on_inbound: F,
    ) -> AddrEventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
        F: FnMut((Msg, SocketAddr, mpsc::Sender<Vec<u8>>)) -> R + Send + 'static,
        R: Future<Output = Result<(), Box<dyn std::error::Error>>> + Send,
    {
        let mut connections: HashMap<SocketAddr, EventManager> = HashMap::new();

        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                if let Some(stream) = connections.get_mut(&addr) {
                    if let Err(x) = stream.send(msg).await {
                        error!("Failed to send to {}", addr);
                    }
                }
            }
        });

        let handle_2 = handle.clone();
        let inbound_handle = handle.spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        handle_2.spawn(async {
                            let event_manager = EventManager::for_tcp_stream(
                                handle_2,
                                max_outbound_queue,
                                stream,
                                addr,
                                inbound_wire,
                                outbound_wire,
                                on_inbound,
                            );

                            connections.insert(addr, event_manager);

                            // Wait for the stream's event manager to exit,
                            // and remove the connection once it does
                            event_manager.join().await;

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

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            outbound_tx: tx,
        }
    }
}

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_inbound<T, F, R>(
    handle: Handle,
    result: Result<(Option<Vec<u8>>, SocketAddr), InboundWireError>,
    sender: mpsc::Sender<T>,
    mut on_inbound: F,
) -> bool
where
    T: Send + 'static,
    F: FnMut((Msg, SocketAddr, mpsc::Sender<T>)) -> R + Send + 'static,
    R: Future<Output = Result<(), Box<dyn std::error::Error>>> + Send,
{
    match result {
        Ok((None, _)) => true,
        Ok((Some(data), addr)) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);

                // Run handler in a new task so we can free up our main
                // event loop
                handle.spawn(async move {
                    if let Err(x) = on_inbound((msg, addr, sender)).await {
                        error!("Encountered error: {}", x);
                    }
                });

                true
            } else {
                warn!("Discarding data of size {} as not valid msg", data.len());
                true
            }
        }
        Err(x) => match x {
            InboundWireError::IO(x) => {
                error!("Fatal IO on wire: {}", x);
                false
            }
            InboundWireError::InputProcessor(x) => {
                error!("Process error on wire: {}", x);
                true
            }
        },
    }
}
