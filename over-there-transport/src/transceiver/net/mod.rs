pub mod tcp;
pub mod udp;

use crate::net;
use over_there_derive::Error;
use std::net::SocketAddr;

pub enum NetTransmission {
    TcpEthernet,
    TcpDialup,
    UdpIpv4,
    UdpIpv6,
}

impl NetTransmission {
    pub fn size(&self) -> usize {
        match self {
            Self::TcpEthernet => net::tcp::MTU_ETHERNET_SIZE,
            Self::TcpDialup => net::tcp::MTU_DIALUP_SIZE,
            Self::UdpIpv4 => net::udp::MAX_IPV4_DATAGRAM_SIZE,
            Self::UdpIpv6 => net::udp::MAX_IPV6_DATAGRAM_SIZE,
        }
    }
}

impl Into<usize> for NetTransmission {
    fn into(self) -> usize {
        self.size()
    }
}

#[derive(Debug, Error)]
pub enum NetSendError {
    Disconnected,
}

pub trait NetSend: Clone {
    fn send(&self, data: &[u8]) -> Result<(), NetSendError>;
    fn addr(&self) -> SocketAddr;
}
