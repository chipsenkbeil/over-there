pub mod tcp;
pub mod udp;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// The Internet Assigned Numbers Authority (IANA) suggested range
/// for dynamic and private ports
///
/// - FreeBSD uses this range since release 4.6
/// - Windows Vista, 7, and Server 2008 use this range
pub const IANA_EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = (49152..=65535);

/// Common Linux kernel port range
pub const LINUX_EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = (32768..=61000);

pub fn make_addr_list(host: IpAddr, ports: Vec<u16>) -> Vec<SocketAddr> {
    ports.iter().map(|p| SocketAddr::new(host, *p)).collect()
}

pub fn make_local_ipv4_addr_list() -> Vec<SocketAddr> {
    make_addr_list(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        IANA_EPHEMERAL_PORT_RANGE.collect(),
    )
}

pub fn make_local_ipv6_addr_list() -> Vec<SocketAddr> {
    make_addr_list(
        IpAddr::from(Ipv6Addr::LOCALHOST),
        IANA_EPHEMERAL_PORT_RANGE.collect(),
    )
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NetTransmission {
    TcpEthernet,
    TcpDialup,
    UdpIpv4,
    UdpIpv6,
}

impl NetTransmission {
    /// Produces transmission size for UDP datagrams based on socket address
    /// being IPv4 or IPv6
    pub fn udp_from_addr(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(_) => Self::UdpIpv4,
            SocketAddr::V6(_) => Self::UdpIpv6,
        }
    }

    pub fn size(self) -> usize {
        match self {
            Self::TcpEthernet => tcp::MTU_ETHERNET_SIZE,
            Self::TcpDialup => tcp::MTU_DIALUP_SIZE,
            Self::UdpIpv4 => udp::MAX_IPV4_DATAGRAM_SIZE,
            Self::UdpIpv6 => udp::MAX_IPV6_DATAGRAM_SIZE,
        }
    }
}

impl Into<usize> for NetTransmission {
    fn into(self) -> usize {
        self.size()
    }
}
