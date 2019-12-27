use std::io::Result;
use std::net::IpAddr;

pub trait Transport {}

pub trait NetworkTransport<T>: Transport
where
    T: Transport,
{
    /// Creates a new instance of a network transport layer, binding to the
    /// specified host using the first open port in the list provided.
    fn bind(host: IpAddr, port: Vec<u16>) -> Result<T>;

    /// Creates a new instance of a network transport layer using default settings
    /// for a local bind
    fn local() -> Result<T>;
}
