pub mod udp;

use super::Transport;
use std::io::Error;
use std::net::{IpAddr, SocketAddr};

pub trait NetworkTransport<T: Transport>: Transport {
    /// Sends a provided data, returning bytes sent
    fn send(&self, data: Vec<u8>, addr: SocketAddr) -> Result<usize, Error>;

    /// Checks for the next incoming data
    fn recv(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Error>;

    /// Creates a new instance of a UDP transport layer, binding to the
    /// specified host using the first open port in the list provided.
    fn bind(host: IpAddr, port: Vec<u16>) -> Result<T, Error>;

    /// Creates a new instance of a UDP transport layer using default settings
    fn local() -> Result<T, Error>;

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
