use crate::Msg;

use log::{error, trace, warn};
use over_there_wire::{Authenticator, Bicrypter, InboundWireError, Wire};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    runtime::Handle,
    sync::{mpsc, Mutex},
    task,
};

pub struct EventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    tx: mpsc::Sender<Vec<u8>>,
}

impl EventManager {
    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), Vec<u8>> {
        self.tx.send(data).await.map_err(|x| x.0)
    }

    pub async fn wait(self) -> Result<(), task::JoinError> {
        tokio::try_join!(self.inbound_handle, self.outbound_handle).map(|_| ())
    }
}

pub struct AddrEventManager {
    inbound_handle: task::JoinHandle<()>,
    outbound_handle: task::JoinHandle<()>,
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
}

impl AddrEventManager {
    pub async fn send_to(
        &mut self,
        data: Vec<u8>,
        addr: SocketAddr,
    ) -> Result<(), (Vec<u8>, SocketAddr)> {
        self.tx.send((data, addr)).await.map_err(|x| x.0)
    }

    pub async fn wait(self) -> Result<(), task::JoinError> {
        tokio::try_join!(self.inbound_handle, self.outbound_handle).map(|_| ())
    }
}

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
        A: Authenticator + Clone + Send + 'static,
        B: Bicrypter + Clone + Send + 'static,
    {
        let (mut reader, mut writer) =
            wire.with_tcp_stream(stream, remote_addr).clone_split();

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(max_outbound_queue);
        let tx_2 = tx.clone();

        // Run IO on same thread
        let inbound_handle = handle.spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(x) = writer.write(&msg).await {
                    error!("Failed to send: {}", x);
                }
            }
        });
        let outbound_handle = handle.spawn(async move {
            loop {
                let tx_3 = tx_2.clone();
                let result = reader.read().await;
                if !process_inbound(result, tx_3, on_inbound_tx.clone()).await {
                    break;
                }
            }
        });

        EventManager {
            inbound_handle,
            outbound_handle,
            tx,
        }
    }
}

impl AddrEventManager {
    pub fn for_udp_socket<A, B>(
        handle: Handle,
        max_outbound_queue: usize,
        socket: UdpSocket,
        wire: Wire<A, B>,
        on_inbound_tx: mpsc::Sender<(
            Msg,
            SocketAddr,
            mpsc::Sender<(Vec<u8>, SocketAddr)>,
        )>,
    ) -> AddrEventManager
    where
        A: Authenticator + Clone + Send + 'static,
        B: Bicrypter + Clone + Send + 'static,
    {
        let (mut reader, mut writer) =
            wire.with_udp_socket(socket).clone_split();

        let (tx, mut rx) =
            mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle = handle.spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                if let Err(x) = writer.write_to(&msg, addr).await {
                    error!("Failed to send: {}", x);
                    break;
                }
            }
        });

        let tx_2 = tx.clone();
        let inbound_handle = handle.spawn(async move {
            loop {
                let tx_3 = tx_2.clone();
                let result = reader.read().await;
                if !process_inbound(result, tx_3, on_inbound_tx.clone()).await {
                    break;
                }
            }
        });

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            tx,
        }
    }

    pub fn for_tcp_listener<A, B>(
        handle: Handle,
        max_outbound_queue: usize,
        mut listener: TcpListener,
        wire: Wire<A, B>,
        on_inbound_tx: mpsc::Sender<(Msg, SocketAddr, mpsc::Sender<Vec<u8>>)>,
    ) -> AddrEventManager
    where
        A: Authenticator + Clone + Send + 'static,
        B: Bicrypter + Clone + Send + 'static,
    {
        let connections: Arc<
            Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>,
        > = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) =
            mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);

        let connections_2 = Arc::clone(&connections);
        let outbound_handle = handle.spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                if let Some(stream) = connections_2.lock().await.get_mut(&addr)
                {
                    if stream.send(msg).await.is_err() {
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
                        let on_inbound_tx_2 = on_inbound_tx.clone();
                        let wire_2 = wire.clone();
                        handle_3.spawn(async move {
                            let event_manager = EventManager::for_tcp_stream(
                                handle_4,
                                max_outbound_queue,
                                stream,
                                addr,
                                wire_2,
                                on_inbound_tx_2,
                            );

                            connections_3
                                .lock()
                                .await
                                .insert(addr, event_manager.tx.clone());

                            // Wait for the stream's event manager to exit,
                            // and remove the connection once it does
                            if let Err(x) = event_manager.wait().await {
                                error!("Event manager exited badly: {}", x);
                            }

                            connections_3.lock().await.remove(&addr);
                        });
                    }
                    Err(x) => {
                        error!(
                            "Listening for connections encountered error: {}",
                            x
                        );
                        break;
                    }
                }
            }
        });

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            tx,
        }
    }
}

/// Process result of receiving data, indicating whether should continue
/// processing additional data
async fn process_inbound<T>(
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
            trace!("Incoming data of size {} from {}", data.len(), addr);
            if let Ok(msg) = Msg::from_slice(&data) {
                trace!("Valid msg {:?} from {}", msg, addr);

                if let Err(x) = on_inbound_tx.send((msg, addr, sender)).await {
                    error!("Encountered error: {}", x);
                }

                true
            } else {
                warn!(
                    "Discarding data of size {} as not valid msg",
                    data.len()
                );
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
