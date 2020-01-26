mod listen;
pub mod route;
pub mod state;

use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::TransceiverThread;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Server {
    state: Arc<Mutex<state::ServerState>>,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<(Vec<u8>, SocketAddr), ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Server {
    pub fn tcp_connect<A, B>(
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
        listen::tcp_listen(host, port, packet_ttl, authenticator, bicrypter)
    }

    pub fn udp_connect<A, B>(
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
        listen::udp_listen(host, port, packet_ttl, authenticator, bicrypter)
    }
}
