pub mod udp;

use std::io::Error;
use std::net::{IpAddr, SocketAddr};

pub trait NetworkTransport {
    /// Sends a provided data, returning bytes sent
    fn send_data(&self, data: Vec<u8>, addr: SocketAddr) -> Result<usize, Error>;

    /// Checks for the next incoming data
    fn recv_data(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Error>;

    /// Retrieves the full address associated with the transport layer
    /// *Usually the full local address*
    fn addr(&self) -> Result<SocketAddr, Error>;

    /// Retrieves the IP address associated with the transport layer
    /// *Usually the local IP address*
    fn ip(&self) -> Result<IpAddr, Error>;

    /// Retrieves the port associated with the transport layer
    /// *Usually the local port*
    fn port(&self) -> Result<u16, Error>;
}
