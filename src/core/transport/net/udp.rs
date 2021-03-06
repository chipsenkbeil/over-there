use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

/// IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

/// IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

pub fn bind(host: IpAddr, port: Vec<u16>) -> io::Result<UdpSocket> {
    let addr_candidates = super::make_addr_list(host, port);
    UdpSocket::bind(&addr_candidates[..])
}

/// Connects to a remote address by binding to a local, ephemeral port
/// and then issuing connect(...) on the socket to filter out all
/// data not coming from the specified address
///
/// NOTE: This seems to be equivalent to a non-bound socket doing a connect,
///       which could look like UdpSocket::bind("0.0.0.0:0").connect(...)
pub fn connect(addr: SocketAddr) -> io::Result<UdpSocket> {
    let socket = if addr.is_ipv4() {
        bind(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            super::IANA_EPHEMERAL_PORT_RANGE.collect(),
        )?
    } else {
        bind(
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            super::IANA_EPHEMERAL_PORT_RANGE.collect(),
        )?
    };

    // NOTE: There appears to be a limitation in MacOS that causes subsequent
    //       calls to socket.send_to(...) after a connect to fail with OS
    //       error code 56 (socket already connected); so, we will NOT perform
    //       a connect if on MacOS
    #[cfg(not(target_os = "macos"))]
    socket.connect(addr)?;

    Ok(socket)
}

pub fn local() -> io::Result<UdpSocket> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        super::IANA_EPHEMERAL_PORT_RANGE.collect(),
    )
}
