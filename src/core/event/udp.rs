use super::AddrEventManager;
use crate::core::Msg;

use log::error;
use crate::wire::{
    Authenticator, Bicrypter, Decrypter, Encrypter, Signer,
    UdpSocketInboundWire, UdpSocketOutboundWire, Verifier, Wire,
};
use std::net::SocketAddr;
use tokio::{net::UdpSocket, runtime::Handle, sync::mpsc};

/// Implementation of AddrEventManager for UDP stream
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
        A: Authenticator + Send + Sync + 'static,
        B: Bicrypter + Send + Sync + 'static,
    {
        let (reader, writer) = wire.with_udp_socket(socket).arc_split();

        let (tx, rx) =
            mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle =
            handle.spawn(udp_socket_outbound_loop(rx, writer));
        let inbound_handle = handle.spawn(udp_socket_inbound_loop(
            tx.clone(),
            reader,
            on_inbound_tx,
        ));

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            tx,
        }
    }
}

impl AddrEventManager {
    // NOTE: This explicit naming only exists as specialization is unstable
    pub fn for_udp_socket_with_cloneable_wire<A, B>(
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
        A: Authenticator + Send + Sync + Clone + 'static,
        B: Bicrypter + Send + Sync + Clone + 'static,
    {
        let (reader, writer) = wire.with_udp_socket(socket).clone_split();

        let (tx, rx) =
            mpsc::channel::<(Vec<u8>, SocketAddr)>(max_outbound_queue);
        let outbound_handle =
            handle.spawn(udp_socket_outbound_loop(rx, writer));
        let inbound_handle = handle.spawn(udp_socket_inbound_loop(
            tx.clone(),
            reader,
            on_inbound_tx,
        ));

        AddrEventManager {
            outbound_handle,
            inbound_handle,
            tx,
        }
    }
}

async fn udp_socket_outbound_loop<S, E>(
    mut rx: mpsc::Receiver<(Vec<u8>, SocketAddr)>,
    mut writer: UdpSocketOutboundWire<S, E>,
) where
    S: Signer,
    E: Encrypter,
{
    while let Some((msg, addr)) = rx.recv().await {
        if let Err(x) = writer.write_to(&msg, addr).await {
            error!("Failed to send: {}", x);
            break;
        }
    }
}

async fn udp_socket_inbound_loop<V, D>(
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
    mut reader: UdpSocketInboundWire<V, D>,
    on_inbound_tx: mpsc::Sender<(
        Msg,
        SocketAddr,
        mpsc::Sender<(Vec<u8>, SocketAddr)>,
    )>,
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
