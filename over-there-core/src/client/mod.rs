pub mod state;

use over_there_auth::NoopAuthenticator;
use over_there_crypto::NoopBicrypter;
use over_there_transport::{
    net, NetStream, NetTransmission, TcpStreamTransceiver, TransceiverContext,
};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub enum Connection {
    /// Connects to a remote TCP port
    Tcp,

    /// Connects to a remote UDP port using
    /// an arbitrary local UDP port
    Udp,

    /// Connects to a remote UDP port using
    /// the provided local UDP port
    UdpVia(SocketAddr),
}

pub fn connect<NS: NetStream>(remote_addr: SocketAddr, connection: Connection) -> Client<NS> {
    match connection {
        Connection::Tcp => Client {
            state: state::State::default(),
            ns: TcpStreamTransceiver::new(
                TcpStream::connect(remote_addr),
                TransceiverContext::new(
                    NetTransmission::TcpEthernet.into(),
                    Duration::from_secs(TransceiverContext::DEFAULT_TTL_IN_SECS),
                    NoopAuthenticator,
                    NoopBicrypter,
                ),
            ),
        },
    }
}

pub struct Client<NS: NetStream> {
    state: state::State,
    ns: NS,
}
