pub mod state;

use crate::msg::{Msg, MsgError};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_transport::{
    net, NetStream, NetTransmission, TcpStreamTransceiver, TransceiverContext, TransceiverThread,
    UdpStreamTransceiver, UdpTransceiver,
};
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub struct Client {
    state: state::State,
    thread: TransceiverThread<Vec<u8>, ()>,
}

impl Client {
    pub fn heartbeat(&self) {}
}

pub fn tcp_connect<A, B>(
    remote_addr: SocketAddr,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    Ok(Client {
        state: state::State::default(),
        thread: TcpStreamTransceiver::new(
            TcpStream::connect(remote_addr)?,
            TransceiverContext::new(
                NetTransmission::TcpEthernet.into(),
                packet_ttl,
                authenticator,
                bicrypter,
            ),
        )
        .spawn(Duration::from_millis(1), |data, responder| {
            if let Ok(msg) = Msg::from_slice(&data) {}
        })?,
    })
}

pub fn udp_connect<A, B>(
    remote_addr: SocketAddr,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
) -> Result<Client, io::Error>
where
    A: Signer + Verifier + Send + Sync + 'static,
    B: Encrypter + Decrypter + Send + Sync + 'static,
{
    let ctx = if remote_addr.is_ipv4() {
        TransceiverContext::new(
            NetTransmission::UdpIpv4.into(),
            packet_ttl,
            authenticator,
            bicrypter,
        )
    } else {
        TransceiverContext::new(
            NetTransmission::UdpIpv6.into(),
            packet_ttl,
            authenticator,
            bicrypter,
        )
    };

    Ok(Client {
        state: state::State::default(),
        thread: UdpTransceiver::new(net::udp::connect(remote_addr)?, ctx)
            .connect(remote_addr)?
            .spawn(Duration::from_millis(1), |data, responder| {
                if let Ok(msg) = Msg::from_slice(&data) {}
            })?,
    })
}
