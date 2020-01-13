use std::io::{self, Result};
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

/// Produces a new function to send data using a udp socket
pub fn new_send_func(socket: UdpSocket, addr: SocketAddr) -> impl FnMut(&[u8]) -> Result<usize> {
    move |data| {
        // TODO: Support sending remaining bytes in loop? Would need to
        //       support collecting bytes for a packet in multiple receives,
        //       which means we'd need a start and stop indicator of some
        //       kind that is a single byte. Seems too complicated, so
        //       easier to fail and give a reason if we don't send all
        //       of the bytes in one go. It's one of the reasons we made
        //       packets of a guaranteed max size.
        let size = socket.send_to(&data, addr)?;
        if size < data.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Only sent {} bytes out of {}", size, data.len()),
            ));
        }

        Ok(size)
    }
}

/// Produces a new function to receive data using a udp socket
pub fn new_recv_func(socket: UdpSocket) -> impl FnMut(&mut [u8]) -> Result<(usize, SocketAddr)> {
    move |data| socket.recv_from(data)
}
