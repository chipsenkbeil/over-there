use crate::Msg;

use log::{error, trace, warn};
use over_there_wire::{
    Decrypter, Encrypter, InboundWire, InboundWireError, OutboundWire, OutboundWireError, Signer,
    Verifier,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io,
    net::{self, TcpListener, TcpStream, UdpSocket},
    runtime::Handle,
    sync::{mpsc, Mutex},
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

    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), Vec<u8>> {
        self.outbound_tx.send(data).await.map_err(|x| x.0)
    }

    pub async fn join(self) {
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
        &mut self,
        data: Vec<u8>,
        addr: SocketAddr,
    ) -> Result<(), (Vec<u8>, SocketAddr)> {
        self.outbound_tx.send((data, addr)).await.map_err(|x| x.0)
    }

    pub async fn join(self) {
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
        let data = self.inbound_wire.process(&buf[..size])?;

        Ok((data, self.remote_addr))
    }
}

struct OutboundTcpStream<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    outbound_wire: OutboundWire<S, E>,
    stream: io::WriteHalf<TcpStream>,
    remote_addr: SocketAddr,
}

impl<S, E> OutboundTcpStream<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    pub async fn write(&mut self, buf: &[u8]) -> Result<(), OutboundWireError> {
        use tokio::io::AsyncWriteExt;

        let data = self.outbound_wire.process(buf)?;

        for packet_bytes in data.iter() {
            let size = self
                .stream
                .write(packet_bytes)
                .await
                .map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
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
        let data = self.inbound_wire.process(&buf[..size])?;

        Ok((data, addr))
    }
}

struct OutboundUdpSocket<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    outbound_wire: OutboundWire<S, E>,
    socket: net::udp::SendHalf,
}

impl<S, E> OutboundUdpSocket<S, E>
where
    S: Signer + Send + 'static,
    E: Encrypter + Send + 'static,
{
    pub async fn write_to(
        &mut self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Result<(), OutboundWireError> {
        let data = self.outbound_wire.process(buf)?;

        for packet_bytes in data.iter() {
            let size = self
                .socket
                .send_to(packet_bytes, &addr)
                .await
                .map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
    }
}

impl EventManager {
    pub fn for_tcp_stream<S, V, E, D>(
        handle: Handle,
        max_outbound_queue: usize,
        stream: TcpStream,
        remote_addr: SocketAddr,
        inbound_wire: InboundWire<V, D>,
        outbound_wire: OutboundWire<S, E>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    ) -> EventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
    {
        // NOTE: Using io::split instead of TcpStream.split as the TcpStream
        //       uses a mutable reference (instead of taking ownership) and
        //       the returned halves are restricted to the stream's lifetime,
        //       meaning you can only run them on a single thread together
        let (stream_reader, stream_writer) = io::split(stream);

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            let mut writer = OutboundTcpStream {
                outbound_wire,
                stream: stream_writer,
                remote_addr,
            };

            while let Some(msg) = rx.recv().await {
                if let Err(x) = writer.write(&msg).await {
                    error!("Failed to send: {}", x);
                }
            }
        });

        let handle_2 = handle.clone();
        let tx_2 = tx.clone();
        let inbound_handle = handle.spawn(async move {
            let mut reader = InboundTcpStream {
                inbound_wire,
                stream: stream_reader,
                remote_addr,
            };

            loop {
                let handle_3 = handle_2.clone();
                let tx_3 = tx_2.clone();
                let result = reader.read().await;
                if !process_inbound(handle_3, result, tx_3, on_inbound_tx.clone()).await {
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
    pub fn for_udp_socket<S, V, E, D>(
        handle: Handle,
        max_outbound_queue: usize,
        socket: UdpSocket,
        inbound_wire: InboundWire<V, D>,
        outbound_wire: OutboundWire<S, E>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<(Vec<u8>, SocketAddr)>)>,
    ) -> AddrEventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
    {
        // NOTE: While this consumes the socket in v0.2.11 of Tokio, this appears
        //       to be a mistake and will be changed to &mut self in v0.3 as
        //       described here:
        //
        //       https://github.com/tokio-rs/tokio/pull/1630#issuecomment-559921381
        //
        let (socket_reader, socket_writer) = socket.split();

        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            let mut writer = OutboundUdpSocket {
                outbound_wire,
                socket: socket_writer,
            };
            while let Some((msg, addr)) = rx.recv().await {
                if let Err(x) = writer.write_to(&msg, addr).await {
                    error!("Failed to send: {}", x);
                    break;
                }
            }
        });

        let handle_2 = handle.clone();
        let tx_2 = tx.clone();
        let inbound_handle = handle.spawn(async move {
            let mut reader = InboundUdpSocket {
                inbound_wire,
                socket: socket_reader,
            };

            loop {
                let handle_3 = handle_2.clone();
                let tx_3 = tx_2.clone();
                let result = reader.read().await;
                if !process_inbound(handle_3, result, tx_3, on_inbound_tx.clone()).await {
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

    pub fn for_tcp_listener<S, V, E, D>(
        handle: Handle,
        max_outbound_queue: usize,
        mut listener: TcpListener,
        inbound_wire: InboundWire<V, D>,
        outbound_wire: OutboundWire<S, E>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    ) -> AddrEventManager
    where
        S: Signer + Send + 'static,
        V: Verifier + Send + 'static,
        E: Encrypter + Send + 'static,
        D: Decrypter + Send + 'static,
    {
        let connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);

        let connections_2 = Arc::clone(&connections);
        let outbound_handle = handle.spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                if let Some(stream) = connections_2.lock().await.get_mut(&addr) {
                    if let Err(_) = stream.send(msg).await {
                        error!("Failed to send to {}", addr);
                    }
                }
            }
        });

        let handle_2 = handle.clone();
        let inbound_handle = handle.spawn(async move {
            let max_outbound_queue = max_outbound_queue;
            loop {
                let connections_3 = Arc::clone(&connections);
                let handle_3 = handle_2.clone();
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let handle_4 = handle_3.clone();
                        let inbound_wire_2 = inbound_wire.clone();
                        let outbound_wire_2 = outbound_wire.clone();
                        let on_inbound_tx_2 = on_inbound_tx.clone();
                        handle_3.spawn(async move {
                            let event_manager = EventManager::for_tcp_stream(
                                handle_4,
                                max_outbound_queue,
                                stream,
                                addr,
                                inbound_wire_2,
                                outbound_wire_2,
                                on_inbound_tx_2,
                            );

                            connections_3
                                .lock()
                                .await
                                .insert(addr, event_manager.outbound_tx().clone());

                            // Wait for the stream's event manager to exit,
                            // and remove the connection once it does
                            event_manager.join().await;

                            connections_3.lock().await.remove(&addr);
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
async fn process_inbound<T>(
    handle: Handle,
    result: Result<(Option<Vec<u8>>, SocketAddr), InboundWireError>,
    sender: mpsc::Sender<T>,
    mut on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<T>)>,
) -> bool
where
    T: Send + 'static,
{
    match result {
        Ok((None, _)) => true,
        Ok((Some(data), addr)) => {
            trace!("Incoming data of size {}", data.len());
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Forwarding {:?} using {:?}", msg, addr);

                if let Err(x) = on_inbound_tx.send((msg, addr, sender)).await {
                    error!("Encountered error: {}", x);
                }

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
