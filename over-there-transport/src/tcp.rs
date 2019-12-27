use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

/// Maximum Transmission Unit for Ethernet in bytes
pub const MTU_ETHERNET_SIZE: usize = 1500;

/// Maximum Transmission Unit for Dialup in bytes
pub const MTU_DIALUP_SIZE: usize = 576;

/// 8080-8099
pub const DEFAULT_PORT_RANGE: std::ops::Range<u16> = (8080..8099);

pub fn bind(host: IpAddr, port: Vec<u16>) -> Result<TcpListener> {
    let addr_candidates: Vec<SocketAddr> = port.iter().map(|p| SocketAddr::new(host, *p)).collect();
    TcpListener::bind(&addr_candidates[..])
}

pub fn local() -> Result<TcpListener> {
    bind(
        IpAddr::from(Ipv4Addr::LOCALHOST),
        DEFAULT_PORT_RANGE.collect(),
    )
}
