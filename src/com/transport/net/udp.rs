use super::{NetworkTransport, Transport};
use log::debug;
use std::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

pub struct UDP {
    /// Internal socket used for communication
    sock: UdpSocket,
}

impl UDP {
    // IPv4 :: 508 = 576 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV4_DATAGRAM_SIZE: usize = 508;

    // IPv6 :: 1212 = 1280 - 60 (IP header) - 8 (udp header)
    pub const MAX_IPV6_DATAGRAM_SIZE: usize = 1212;

    // 60001–61000
    pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (60001..61000);

    pub fn new(sock: UdpSocket) -> UDP {
        UDP { sock }
    }
}

impl Transport for UDP {}

impl NetworkTransport<UDP> for UDP {
    /// Creates a new instance of a UDP transport layer, binding to the
    /// specified host using the first open port in the list provided.
    fn bind(host: IpAddr, port: Vec<u16>) -> Result<Self, Error> {
        let addr_candidates: Vec<SocketAddr> =
            port.iter().map(|p| SocketAddr::new(host, *p)).collect();
        let sock = UdpSocket::bind(&addr_candidates[..])?;
        let instance = UDP::new(sock);
        Ok(instance)
    }

    /// Creates a new instance of a UDP transport layer using default settings
    fn local() -> Result<Self, Error> {
        Self::bind(
            IpAddr::from(Ipv4Addr::LOCALHOST),
            UDP::DEFAULT_PORT_RANGE.collect(),
        )
    }

    fn addr(&self) -> Result<SocketAddr, Error> {
        self.sock.local_addr()
    }

    fn ip(&self) -> Result<IpAddr, Error> {
        let addr = self.addr()?;
        Ok(addr.ip())
    }

    fn port(&self) -> Result<u16, Error> {
        let addr = self.addr()?;
        Ok(addr.port())
    }

    /// Sends a provided data, returning bytes sent
    fn send(&self, data: Vec<u8>, addr: SocketAddr) -> Result<usize, Error> {
        self.sock.send_to(&data, addr)
    }

    /// Checks for the next incoming data
    fn recv(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let (bsize, src) = self.sock.recv_from(buffer)?;
        debug!("Received {} bytes", bsize);
        Ok((bsize, src))
    }
}
