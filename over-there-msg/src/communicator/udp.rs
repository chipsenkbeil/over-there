use crate::{
    communicator::{self, Communicator},
    msg::Msg,
};
use over_there_transport::{NetworkTransport, UDPTransport};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub fn new_udp(
    host: IpAddr,
    port: Option<Vec<u16>>,
) -> Result<Communicator<UDPTransport>, io::Error> {
    match host {
        IpAddr::V4(addr) => new_udp_ipv4(addr, port),
        IpAddr::V6(addr) => new_udp_ipv6(addr, port),
    }
}

pub fn new_udp_ipv4(
    host: Ipv4Addr,
    port: Option<Vec<u16>>,
) -> Result<Communicator<UDPTransport>, io::Error> {
    Ok(Communicator::from_transport(
        UDPTransport::bind(
            IpAddr::V4(host),
            port.unwrap_or(UDPTransport::DEFAULT_PORT_RANGE.collect()),
        )?,
        UDPTransport::MAX_IPV4_DATAGRAM_SIZE as u32,
    ))
}

pub fn new_udp_ipv6(
    host: Ipv6Addr,
    port: Option<Vec<u16>>,
) -> Result<Communicator<UDPTransport>, io::Error> {
    Ok(Communicator::from_transport(
        UDPTransport::bind(
            IpAddr::V6(host),
            port.unwrap_or(UDPTransport::DEFAULT_PORT_RANGE.collect()),
        )?,
        UDPTransport::MAX_IPV6_DATAGRAM_SIZE as u32,
    ))
}

impl Communicator<UDPTransport> {
    pub fn send(&self, msg: Msg, addr: SocketAddr) -> Result<(), communicator::Error> {
        self.transmitter
            .send(msg, |data| {
                self.transport.socket().send_to(&data, addr).map(|_| ())
            })
            .map_err(communicator::Error::Transmitter)
    }

    pub fn recv(&self) -> Result<Option<(Msg, SocketAddr)>, communicator::Error> {
        let mut addr: Option<SocketAddr> = None;
        let msg = self
            .transmitter
            .recv(|buf| {
                let (size, src) = self.transport.socket().recv_from(buf)?;
                addr = Some(src);
                Ok(size)
            })
            .map_err(communicator::Error::Transmitter)?;
        Ok(match (msg, addr) {
            (Some(m), Some(a)) => Some((m, a)),
            _ => None,
        })
    }
}
