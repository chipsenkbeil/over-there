mod action;
mod listen;
pub mod route;
pub mod state;

use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    net, TcpListenerTransceiverError, TransceiverThread, UdpTransceiverError,
};
use std::io;
use std::net::{IpAddr, SocketAddr, TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Server {
    state: Arc<Mutex<state::ServerState>>,

    /// Represents the address the server is bound to
    pub addr: SocketAddr,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<(Vec<u8>, SocketAddr), ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Server {
    pub fn listen_tcp<A, B, C>(
        host: IpAddr,
        port: Vec<u16>,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpListenerTransceiverError) -> bool + Send + 'static,
    {
        Self::listen_using_tcp_listener(
            net::tcp::bind(host, port)?,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn listen_using_tcp_listener<A, B, C>(
        listener: TcpListener,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpListenerTransceiverError) -> bool + Send + 'static,
    {
        listen::tcp_listen(listener, packet_ttl, authenticator, bicrypter, err_callback)
    }

    pub fn listen_udp<A, B, C>(
        host: IpAddr,
        port: Vec<u16>,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpTransceiverError) -> bool + Send + 'static,
    {
        Self::listen_using_udp_socket(
            net::udp::bind(host, port)?,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn listen_using_udp_socket<A, B, C>(
        socket: UdpSocket,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpTransceiverError) -> bool + Send + 'static,
    {
        listen::udp_listen(socket, packet_ttl, authenticator, bicrypter, err_callback)
    }

    pub fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        self.transceiver_thread.join()?;
        self.msg_thread
            .join()
            .map_err(|_| "Msg Process Thread Join Error")?;

        Ok(())
    }
}
