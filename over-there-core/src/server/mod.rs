mod listen;
pub mod route;
pub mod state;

use crate::{
    msg::{callback::Callback, Msg},
    state::State,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{net, TransceiverThread};
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
    pub fn listen_tcp<A, B>(
        host: IpAddr,
        port: Vec<u16>,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        Self::listen_using_tcp_listener(
            net::tcp::bind(host, port)?,
            packet_ttl,
            authenticator,
            bicrypter,
        )
    }

    pub fn listen_using_tcp_listener<A, B>(
        listener: TcpListener,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        listen::tcp_listen(listener, packet_ttl, authenticator, bicrypter)
    }

    pub fn listen_udp<A, B>(
        host: IpAddr,
        port: Vec<u16>,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        Self::listen_using_udp_socket(
            net::udp::bind(host, port)?,
            packet_ttl,
            authenticator,
            bicrypter,
        )
    }

    pub fn listen_using_udp_socket<A, B>(
        socket: UdpSocket,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        listen::udp_listen(socket, packet_ttl, authenticator, bicrypter)
    }

    pub fn add_callback(&mut self, id: u32, callback: impl FnMut(&Msg) + Send + 'static) {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .add_callback(id, callback)
    }

    pub fn take_callback(&mut self, id: u32) -> Option<Box<Callback>> {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .take_callback(id)
    }

    pub fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        self.transceiver_thread.join()?;
        self.msg_thread
            .join()
            .map_err(|_| "Msg Process Thread Join Error")?;

        Ok(())
    }
}
