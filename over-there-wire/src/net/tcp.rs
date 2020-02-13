use std::io;
use std::net::{IpAddr, Ipv4Addr, TcpListener};

/// Maximum Transmission Unit for Ethernet in bytes
pub const MTU_ETHERNET_SIZE: usize = 1500;

/// Maximum Transmission Unit for Dialup in bytes
pub const MTU_DIALUP_SIZE: usize = 576;

pub fn bind(host: IpAddr, port: Vec<u16>) -> io::Result<TcpListener> {
    let addr_candidates = super::make_addr_list(host, port);
    TcpListener::bind(&addr_candidates[..])
}

pub fn local() -> io::Result<TcpListener> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        super::IANA_EPHEMERAL_PORT_RANGE.collect(),
    )
}
