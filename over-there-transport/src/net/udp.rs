use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

/// IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

/// IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

/// 60001–61000
pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (60001..61000);

pub fn bind(host: IpAddr, port: Vec<u16>) -> Result<UdpSocket> {
    let addr_candidates: Vec<SocketAddr> = port.iter().map(|p| SocketAddr::new(host, *p)).collect();
    UdpSocket::bind(&addr_candidates[..])
}

pub fn local() -> Result<UdpSocket> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        DEFAULT_PORT_RANGE.collect(),
    )
}
