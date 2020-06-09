mod connected;
pub mod error;
pub mod file;
mod inbound;
pub mod proc;
pub mod state;

pub use connected::ConnectedClient;

use crate::core::{
    event::{AddrEventManager, EventManager},
    msg::content::Content,
    Transport,
};
use derive_builder::Builder;
use log::warn;
use crate::utils::Either;
use crate::transport::{
    self as wire, Authenticator, Bicrypter, NetTransmission, Wire,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::{
    io,
    net::{TcpStream, UdpSocket},
    runtime::Handle,
    sync::{mpsc, Mutex},
};

/// Represents a client configuration prior to connecting
#[derive(Builder)]
pub struct Client<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    /// TTL to collect all packets for a msg
    #[builder(default = "crate::transport::constants::DEFAULT_TTL")]
    packet_ttl: Duration,

    /// Used to sign & verify msgs
    authenticator: A,

    /// Used to encrypt & decrypt msgs
    bicrypter: B,

    /// Transportation mechanism & address to listen on
    transport: Transport,

    /// Internal buffer for cross-thread messaging
    #[builder(default = "1000")]
    buffer: usize,
}

impl<A, B> Client<A, B>
where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    /// Starts actively listening for msgs via the specified transport medium
    pub async fn connect(self) -> io::Result<ConnectedClient> {
        let state = Arc::new(Mutex::new(state::ClientState::default()));

        match self.transport.clone() {
            Transport::Tcp(addrs) => {
                build_and_connect_tcp_client(self, Arc::clone(&state), &addrs)
                    .await
            }
            Transport::Udp(addrs) => {
                build_and_connect_udp_client(self, Arc::clone(&state), &addrs)
                    .await
            }
        }
    }
}

async fn build_and_connect_tcp_client<A, B>(
    client: Client<A, B>,
    state: Arc<Mutex<state::ClientState>>,
    addrs: &[SocketAddr],
) -> io::Result<ConnectedClient>
where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    let handle = Handle::current();

    // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
    //       so we have to loop through manually
    // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
    let stream = {
        let mut stream = None;
        for addr in addrs.iter() {
            match TcpStream::connect(addr).await {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(x) => warn!("Failed to connect to {}: {}", addr, x),
            }
        }
        stream
            .ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionRefused))?
    };
    let remote_addr = stream.peer_addr()?;
    let wire = Wire::new(
        NetTransmission::TcpEthernet.into(),
        client.packet_ttl,
        client.authenticator,
        client.bicrypter,
    );

    let (tx, rx) = mpsc::channel(client.buffer);
    let event_handle = handle.spawn(event_loop(
        Arc::clone(&state),
        inbound::InboundMsgReader::new(rx),
    ));
    let event_manager = EventManager::for_tcp_stream(
        handle.clone(),
        client.buffer,
        stream,
        remote_addr,
        wire,
        tx,
    );

    Ok(ConnectedClient {
        state,
        event_manager: Either::Left(event_manager),
        event_handle,
        remote_addr,
        timeout: ConnectedClient::DEFAULT_TIMEOUT,
    })
}

async fn build_and_connect_udp_client<A, B>(
    client: Client<A, B>,
    state: Arc<Mutex<state::ClientState>>,
    addrs: &[SocketAddr],
) -> io::Result<ConnectedClient>
where
    A: Authenticator + Send + Sync + 'static,
    B: Bicrypter + Send + Sync + 'static,
{
    let handle = Handle::current();

    // NOTE: Tokio does not support &[SocketAddr] -> ToSocketAddrs,
    //       so we have to loop through manually
    // See https://github.com/tokio-rs/tokio/pull/1760#discussion_r379120864
    let (socket, remote_addr) = {
        let mut socket_and_addr = None;
        for addr in addrs.iter() {
            match wire::net::udp::connect(*addr) {
                Ok(s) => {
                    socket_and_addr = Some((s, *addr));
                    break;
                }
                Err(x) => warn!("Failed to connect to {}: {}", *addr, x),
            }
        }

        // NOTE: Must use Handle::enter to provide proper runtime when
        //       using UdpSocket::from_std
        handle.enter(|| {
            socket_and_addr
                .ok_or_else(|| {
                    io::Error::from(io::ErrorKind::ConnectionRefused)
                })
                .and_then(|(s, addr)| UdpSocket::from_std(s).map(|s| (s, addr)))
        })?
    };

    let addr = socket.local_addr()?;
    let transmission = NetTransmission::udp_from_addr(addr);

    let wire = Wire::new(
        transmission.into(),
        client.packet_ttl,
        client.authenticator,
        client.bicrypter,
    );

    let (tx, rx) = mpsc::channel(client.buffer);
    let event_handle = handle.spawn(event_loop(
        Arc::clone(&state),
        inbound::InboundMsgReader::new(rx),
    ));
    let addr_event_manager = AddrEventManager::for_udp_socket(
        handle,
        client.buffer,
        socket,
        wire,
        tx,
    );

    Ok(ConnectedClient {
        state,
        event_manager: Either::Right(addr_event_manager),
        event_handle,
        remote_addr,
        timeout: ConnectedClient::DEFAULT_TIMEOUT,
    })
}

async fn event_loop<T>(
    state: Arc<Mutex<state::ClientState>>,
    mut r: inbound::InboundMsgReader<T>,
) {
    while let Some(msg) = r.next().await {
        // Update the last time we received a msg from the server
        state.lock().await.last_contact = Instant::now();

        if let (Some(header), Content::Reply(reply)) =
            (msg.parent_header.as_ref(), &msg.content)
        {
            state
                .lock()
                .await
                .callback_manager
                .invoke_callback(header.id, reply)
        }
    }
}
