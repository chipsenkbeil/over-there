use crate::transport::{NetworkTransport, Transport};
use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

pub struct UDPTransport {
    /// Internal socket used for communication
    sock: UdpSocket,
}

impl UDPTransport {
    /// IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

    /// IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

    /// 60001–61000
    pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (60001..61000);

    pub fn new(sock: UdpSocket) -> Self {
        Self { sock }
    }

    pub fn socket(&self) -> &UdpSocket {
        &self.sock
    }
}

impl Transport for UDPTransport {}

impl NetworkTransport<UDPTransport> for UDPTransport {
    fn bind(host: IpAddr, port: Vec<u16>) -> Result<Self> {
        let addr_candidates: Vec<SocketAddr> =
            port.iter().map(|p| SocketAddr::new(host, *p)).collect();
        let sock = UdpSocket::bind(&addr_candidates[..])?;
        let instance = Self::new(sock);
        Ok(instance)
    }

    fn local() -> Result<Self> {
        Self::bind(
            IpAddr::from(Ipv4Addr::LOCALHOST),
            Self::DEFAULT_PORT_RANGE.collect(),
        )
    }
}
